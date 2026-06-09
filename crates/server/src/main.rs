use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

use anyhow::Context;
use axum::{
    Json, Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use geode_td_shared::{
    BoardSnapshot, ClientMessage, GameMode, GridPoint, LobbyPlayer, LobbyState, LobbyStatus,
    LobbySummary, MAX_LOBBY_PLAYERS, ServerMessage, SharedBoardLayout,
};
use rand::{Rng, SeedableRng, rngs::StdRng};
use tokio::sync::{Mutex, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

const CHECKPOINT_COUNT: usize = 4;
const BASE_GRID_COLUMNS: i32 = 29;
const BASE_GRID_ROWS: i32 = 17;
const PLAY_AREA_SCALE: i32 = 4;
const GRID_COLUMNS: i32 = BASE_GRID_COLUMNS * PLAY_AREA_SCALE;
const GRID_ROWS: i32 = BASE_GRID_ROWS * PLAY_AREA_SCALE;

#[derive(Clone)]
struct AppState {
    inner: Arc<Mutex<ServerState>>,
}

#[derive(Default)]
struct ServerState {
    players: HashMap<String, ConnectedPlayer>,
    lobbies: HashMap<String, Lobby>,
}

struct ConnectedPlayer {
    name: String,
    lobby_id: Option<String>,
    tx: mpsc::UnboundedSender<ServerMessage>,
}

struct Lobby {
    id: String,
    name: String,
    host_player_id: String,
    player_ids: Vec<String>,
    status: LobbyStatus,
    mode: Option<GameMode>,
    layout: Option<SharedBoardLayout>,
    snapshots: HashMap<String, BoardSnapshot>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "geode_td_server=info,tower_http=warn".to_string()),
        )
        .init();

    let state = AppState {
        inner: Arc::new(Mutex::new(ServerState::default())),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/lobbies", get(list_lobbies))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = std::env::var("GEODE_TD_SERVER_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:4000".to_string())
        .parse::<SocketAddr>()
        .context("GEODE_TD_SERVER_ADDR must be a socket address")?;

    info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn list_lobbies(State(state): State<AppState>) -> Json<Vec<LobbySummary>> {
    let server = state.inner.lock().await;
    Json(server.lobby_summaries())
}

async fn ws_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(state, socket))
}

async fn handle_socket(state: AppState, socket: WebSocket) {
    let player_id = Uuid::new_v4().to_string();
    let (mut socket_tx, mut socket_rx) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    {
        let mut server = state.inner.lock().await;
        server.players.insert(
            player_id.clone(),
            ConnectedPlayer {
                name: "Player".to_string(),
                lobby_id: None,
                tx: tx.clone(),
            },
        );
    }

    let outbound = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else {
                continue;
            };
            if socket_tx.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    send_to(
        &tx,
        ServerMessage::Welcome {
            player_id: player_id.clone(),
        },
    );

    while let Some(result) = socket_rx.next().await {
        let message = match result {
            Ok(message) => message,
            Err(err) => {
                warn!(player_id, "websocket receive error: {err}");
                break;
            }
        };

        match message {
            Message::Text(text) => {
                let parsed = serde_json::from_str::<ClientMessage>(&text);
                match parsed {
                    Ok(message) => handle_client_message(&state, &player_id, message).await,
                    Err(err) => {
                        let server = state.inner.lock().await;
                        server.send_to_player(
                            &player_id,
                            ServerMessage::Error {
                                message: format!("Invalid message: {err}"),
                            },
                        );
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => {}
        }
    }

    cleanup_player(&state, &player_id).await;
    outbound.abort();
}

async fn handle_client_message(state: &AppState, player_id: &str, message: ClientMessage) {
    let mut server = state.inner.lock().await;

    match message {
        ClientMessage::ListLobbies => {
            let summaries = server.lobby_summaries();
            server.send_to_player(player_id, ServerMessage::LobbyList { lobbies: summaries });
        }
        ClientMessage::CreateLobby {
            player_name,
            lobby_name,
        } => {
            server.leave_lobby(player_id);
            server.set_player_name(player_id, player_name);

            let lobby_id = short_id();
            let lobby = Lobby {
                id: lobby_id.clone(),
                name: sanitize_lobby_name(lobby_name, player_id, &server),
                host_player_id: player_id.to_string(),
                player_ids: vec![player_id.to_string()],
                status: LobbyStatus::Waiting,
                mode: Some(GameMode::Standard),
                layout: None,
                snapshots: HashMap::new(),
            };
            server.lobbies.insert(lobby_id.clone(), lobby);
            if let Some(player) = server.players.get_mut(player_id) {
                player.lobby_id = Some(lobby_id.clone());
            }

            server.broadcast_lobby_state(&lobby_id);
            server.broadcast_lobby_list();
        }
        ClientMessage::JoinLobby {
            lobby_id,
            player_name,
        } => {
            server.leave_lobby(player_id);
            server.set_player_name(player_id, player_name);

            let Some(lobby) = server.lobbies.get_mut(&lobby_id) else {
                server.send_error(player_id, "Lobby not found.");
                return;
            };
            if lobby.status != LobbyStatus::Waiting {
                server.send_error(player_id, "That lobby has already started.");
                return;
            }
            if lobby.player_ids.len() >= MAX_LOBBY_PLAYERS
                && !lobby.player_ids.iter().any(|id| id == player_id)
            {
                server.send_error(player_id, "That lobby is full.");
                return;
            }
            if !lobby.player_ids.iter().any(|id| id == player_id) {
                lobby.player_ids.push(player_id.to_string());
            }
            if let Some(player) = server.players.get_mut(player_id) {
                player.lobby_id = Some(lobby_id.clone());
            }

            let joined = server.lobby_player(player_id);
            if let Some(player) = joined {
                server.broadcast_to_lobby(&lobby_id, ServerMessage::PlayerJoined { player });
            }
            server.broadcast_lobby_state(&lobby_id);
            server.broadcast_lobby_list();
        }
        ClientMessage::LeaveLobby => {
            server.leave_lobby(player_id);
            server.broadcast_lobby_list();
        }
        ClientMessage::SetMode { mode } => {
            let Some(lobby_id) = server.player_lobby_id(player_id) else {
                server.send_error(player_id, "You are not in a lobby.");
                return;
            };
            let Some(lobby) = server.lobbies.get_mut(&lobby_id) else {
                server.send_error(player_id, "Lobby not found.");
                return;
            };
            if lobby.host_player_id != player_id {
                server.send_error(player_id, "Only the host can set the mode.");
                return;
            }
            if lobby.status != LobbyStatus::Waiting {
                server.send_error(player_id, "The game has already started.");
                return;
            }
            lobby.mode = Some(mode);
            server.broadcast_lobby_state(&lobby_id);
        }
        ClientMessage::StartGame => {
            let Some(lobby_id) = server.player_lobby_id(player_id) else {
                server.send_error(player_id, "You are not in a lobby.");
                return;
            };
            let Some(lobby) = server.lobbies.get_mut(&lobby_id) else {
                server.send_error(player_id, "Lobby not found.");
                return;
            };
            if lobby.host_player_id != player_id {
                server.send_error(player_id, "Only the host can start the game.");
                return;
            }
            let mode = lobby.mode.clone().unwrap_or(GameMode::Standard);
            let layout = shared_layout(mode);
            lobby.status = LobbyStatus::InGame;
            lobby.layout = Some(layout.clone());
            server.broadcast_to_lobby(&lobby_id, ServerMessage::GameStarted { layout });
            server.broadcast_lobby_state(&lobby_id);
        }
        ClientMessage::BoardSnapshot { snapshot } => {
            let Some(lobby_id) = server.player_lobby_id(player_id) else {
                server.send_error(player_id, "You are not in a lobby.");
                return;
            };
            let Some(lobby) = server.lobbies.get_mut(&lobby_id) else {
                server.send_error(player_id, "Lobby not found.");
                return;
            };
            if lobby.status != LobbyStatus::InGame {
                server.send_error(player_id, "The game has not started.");
                return;
            }
            lobby
                .snapshots
                .insert(player_id.to_string(), snapshot.clone());
            server.broadcast_to_lobby_except(
                &lobby_id,
                player_id,
                ServerMessage::PlayerSnapshot {
                    player_id: player_id.to_string(),
                    snapshot,
                },
            );
        }
    }
}

async fn cleanup_player(state: &AppState, player_id: &str) {
    let mut server = state.inner.lock().await;
    server.leave_lobby(player_id);
    server.players.remove(player_id);
}

impl ServerState {
    fn set_player_name(&mut self, player_id: &str, name: String) {
        if let Some(player) = self.players.get_mut(player_id) {
            let trimmed = name.trim();
            player.name = if trimmed.is_empty() {
                "Player".to_string()
            } else {
                trimmed.chars().take(32).collect()
            };
        }
    }

    fn player_lobby_id(&self, player_id: &str) -> Option<String> {
        self.players.get(player_id)?.lobby_id.clone()
    }

    fn leave_lobby(&mut self, player_id: &str) {
        let Some(lobby_id) = self.player_lobby_id(player_id) else {
            return;
        };

        if let Some(player) = self.players.get_mut(player_id) {
            player.lobby_id = None;
        }

        let mut removed_lobby = false;
        if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
            lobby.player_ids.retain(|id| id != player_id);
            lobby.snapshots.remove(player_id);

            if lobby.player_ids.is_empty() {
                removed_lobby = true;
            } else if lobby.host_player_id == player_id {
                lobby.host_player_id = lobby.player_ids[0].clone();
            }
        }

        if removed_lobby {
            self.lobbies.remove(&lobby_id);
            self.broadcast_lobby_list();
        } else {
            self.broadcast_to_lobby(
                &lobby_id,
                ServerMessage::PlayerLeft {
                    player_id: player_id.to_string(),
                },
            );
            self.broadcast_lobby_state(&lobby_id);
            self.broadcast_lobby_list();
        }
    }

    fn lobby_summaries(&self) -> Vec<LobbySummary> {
        let mut summaries = self
            .lobbies
            .values()
            .map(|lobby| LobbySummary {
                id: lobby.id.clone(),
                name: lobby.name.clone(),
                host_player_id: lobby.host_player_id.clone(),
                host_name: self
                    .players
                    .get(&lobby.host_player_id)
                    .map(|player| player.name.clone())
                    .unwrap_or_else(|| "Host".to_string()),
                player_count: lobby.player_ids.len(),
                max_players: MAX_LOBBY_PLAYERS,
                status: lobby.status.clone(),
                mode: lobby.mode.clone(),
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| left.id.cmp(&right.id));
        summaries
    }

    fn lobby_state(&self, lobby_id: &str) -> Option<LobbyState> {
        let lobby = self.lobbies.get(lobby_id)?;
        let players = lobby
            .player_ids
            .iter()
            .filter_map(|id| self.lobby_player(id))
            .collect();

        Some(LobbyState {
            id: lobby.id.clone(),
            name: lobby.name.clone(),
            host_player_id: lobby.host_player_id.clone(),
            players,
            max_players: MAX_LOBBY_PLAYERS,
            status: lobby.status.clone(),
            mode: lobby.mode.clone(),
            layout: lobby.layout.clone(),
        })
    }

    fn lobby_player(&self, player_id: &str) -> Option<LobbyPlayer> {
        let player = self.players.get(player_id)?;
        let lobby_id = player.lobby_id.as_ref()?;
        let lobby = self.lobbies.get(lobby_id)?;
        Some(LobbyPlayer {
            id: player_id.to_string(),
            name: player.name.clone(),
            is_host: lobby.host_player_id == player_id,
        })
    }

    fn broadcast_lobby_state(&self, lobby_id: &str) {
        let Some(lobby) = self.lobby_state(lobby_id) else {
            return;
        };
        self.broadcast_to_lobby(lobby_id, ServerMessage::LobbyState { lobby });
    }

    fn broadcast_lobby_list(&self) {
        let lobbies = self.lobby_summaries();
        for player in self.players.values() {
            send_to(
                &player.tx,
                ServerMessage::LobbyList {
                    lobbies: lobbies.clone(),
                },
            );
        }
    }

    fn broadcast_to_lobby(&self, lobby_id: &str, message: ServerMessage) {
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return;
        };
        for player_id in &lobby.player_ids {
            self.send_to_player(player_id, message.clone());
        }
    }

    fn broadcast_to_lobby_except(&self, lobby_id: &str, excluded: &str, message: ServerMessage) {
        let Some(lobby) = self.lobbies.get(lobby_id) else {
            return;
        };
        for player_id in &lobby.player_ids {
            if player_id != excluded {
                self.send_to_player(player_id, message.clone());
            }
        }
    }

    fn send_to_player(&self, player_id: &str, message: ServerMessage) {
        let Some(player) = self.players.get(player_id) else {
            return;
        };
        send_to(&player.tx, message);
    }

    fn send_error(&self, player_id: &str, message: &str) {
        self.send_to_player(
            player_id,
            ServerMessage::Error {
                message: message.to_string(),
            },
        );
    }
}

fn sanitize_lobby_name(lobby_name: String, player_id: &str, server: &ServerState) -> String {
    let trimmed = lobby_name.trim();
    if trimmed.is_empty() {
        let host = server
            .players
            .get(player_id)
            .map(|player| player.name.as_str())
            .unwrap_or("Player");
        format!("{host}'s Lobby")
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn send_to(tx: &mpsc::UnboundedSender<ServerMessage>, message: ServerMessage) {
    if tx.send(message).is_err() {
        error!("failed to send websocket message");
    }
}

fn short_id() -> String {
    Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(8)
        .collect()
}

fn shared_layout(mode: GameMode) -> SharedBoardLayout {
    match mode {
        GameMode::Standard => SharedBoardLayout {
            mode,
            seed: None,
            checkpoints: standard_checkpoints(),
        },
        GameMode::Random => {
            let seed = rand::rng().random::<u64>();
            SharedBoardLayout {
                mode,
                seed: Some(seed),
                checkpoints: random_checkpoints(seed),
            }
        }
    }
}

fn standard_checkpoints() -> Vec<GridPoint> {
    let center = scaled_grid_point(BASE_GRID_COLUMNS / 2, BASE_GRID_ROWS / 2);
    let lower_left = scaled_grid_point(6, 3);
    let lower_right = scaled_grid_point(22, 3);
    let upper_left = scaled_grid_point(6, 13);
    let upper_right = scaled_grid_point(22, 13);

    vec![upper_left, lower_left, lower_right, upper_right, center]
}

fn random_checkpoints(seed: u64) -> Vec<GridPoint> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut checkpoints = Vec::with_capacity(CHECKPOINT_COUNT);
    let mut occupied = HashSet::from([start_point(), finish_point()]);
    let col_margin = 4 * PLAY_AREA_SCALE;
    let row_margin = 2 * PLAY_AREA_SCALE;

    while checkpoints.len() < CHECKPOINT_COUNT {
        let col = rng.random_range(col_margin..(GRID_COLUMNS - col_margin));
        let row = rng.random_range(row_margin..(GRID_ROWS - row_margin));
        let point = GridPoint { col, row };

        if !occupied.insert((point.col, point.row)) {
            continue;
        }

        checkpoints.push(point);
    }

    checkpoints
}

fn scaled_grid_point(col: i32, row: i32) -> GridPoint {
    GridPoint {
        col: col * PLAY_AREA_SCALE,
        row: row * PLAY_AREA_SCALE,
    }
}

fn start_point() -> (i32, i32) {
    (PLAY_AREA_SCALE, (BASE_GRID_ROWS / 2) * PLAY_AREA_SCALE)
}

fn finish_point() -> (i32, i32) {
    (
        (BASE_GRID_COLUMNS - 2) * PLAY_AREA_SCALE,
        (BASE_GRID_ROWS / 2) * PLAY_AREA_SCALE,
    )
}
