use bevy::prelude::*;
use geode_td_shared::{ClientMessage, LobbyState, LobbySummary, ServerMessage, SharedBoardLayout};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

const DEFAULT_SERVER_URL: &str = "ws://127.0.0.1:4000/ws";
const REFRESH_SECONDS: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameIntent {
    Host,
    Join,
}

#[derive(Resource)]
pub struct MultiplayerClient {
    pub server_url: String,
    pub display_name: String,
    pub display_name_input: String,
    pub lobby_name_input: String,
    pub pending_name_intent: Option<NameIntent>,
    pub player_id: Option<String>,
    pub lobbies: Vec<LobbySummary>,
    pub lobby: Option<LobbyState>,
    pub pending_layout: Option<SharedBoardLayout>,
    pub status: String,
    pub list_revision: u64,
    pub lobby_revision: u64,
    refresh_timer: Timer,
    outbound: Option<Sender<ClientMessage>>,
    inbound: Option<Arc<Mutex<Receiver<ServerMessage>>>>,
}

impl Default for MultiplayerClient {
    fn default() -> Self {
        let display_name = load_display_name();
        Self {
            server_url: std::env::var("GEODE_TD_WS_URL")
                .unwrap_or_else(|_| DEFAULT_SERVER_URL.to_string()),
            display_name_input: display_name.clone(),
            display_name,
            lobby_name_input: String::new(),
            pending_name_intent: None,
            player_id: None,
            lobbies: Vec::new(),
            lobby: None,
            pending_layout: None,
            status: "Disconnected".to_string(),
            list_revision: 0,
            lobby_revision: 0,
            refresh_timer: Timer::from_seconds(REFRESH_SECONDS, TimerMode::Repeating),
            outbound: None,
            inbound: None,
        }
    }
}

impl MultiplayerClient {
    pub fn has_display_name(&self) -> bool {
        !self.display_name.trim().is_empty()
    }

    pub fn save_display_name_from_input(&mut self) {
        let name = sanitize_name(&self.display_name_input);
        self.display_name = name;
        self.display_name_input = self.display_name.clone();
        if let Err(err) = save_display_name(&self.display_name) {
            self.status = format!("Could not save display name: {err}");
        }
    }

    pub fn ensure_connected(&mut self) {
        if self.outbound.is_some() {
            return;
        }

        let (outbound_tx, outbound_rx) = mpsc::channel::<ClientMessage>();
        let (inbound_tx, inbound_rx) = mpsc::channel::<ServerMessage>();
        let url = self.server_url.clone();
        self.status = format!("Connecting to {url}");

        spawn_network_thread(url, outbound_rx, inbound_tx);
        self.outbound = Some(outbound_tx);
        self.inbound = Some(Arc::new(Mutex::new(inbound_rx)));
    }

    pub fn request_lobbies(&mut self) {
        self.send(ClientMessage::ListLobbies);
    }

    pub fn create_lobby(&mut self) {
        let lobby_name = sanitize_lobby_name(&self.lobby_name_input, &self.display_name);
        self.lobby_name_input = lobby_name.clone();
        self.lobby = None;
        self.status = "Creating lobby...".to_string();
        self.send(ClientMessage::CreateLobby {
            player_name: self.display_name.clone(),
            lobby_name,
        });
    }

    pub fn join_lobby(&mut self, lobby_id: String) {
        self.lobby = None;
        self.status = "Joining lobby...".to_string();
        self.send(ClientMessage::JoinLobby {
            lobby_id,
            player_name: self.display_name.clone(),
        });
    }

    pub fn leave_lobby(&mut self) {
        self.lobby = None;
        self.lobby_revision += 1;
        self.send_if_connected(ClientMessage::LeaveLobby);
    }

    pub fn start_game(&mut self) {
        self.send(ClientMessage::StartGame);
    }

    pub fn tick_join_refresh(&mut self, time: &Time) {
        self.refresh_timer.tick(time.delta());
        if self.refresh_timer.just_finished() {
            self.request_lobbies();
        }
    }

    fn send(&mut self, message: ClientMessage) {
        self.ensure_connected();
        self.send_if_connected(message);
    }

    fn send_if_connected(&mut self, message: ClientMessage) {
        let Some(outbound) = &self.outbound else {
            return;
        };
        if outbound.send(message).is_err() {
            self.status = "Connection closed. Reopen Multiplayer to reconnect.".to_string();
            self.outbound = None;
            self.inbound = None;
        }
    }
}

pub fn poll_multiplayer_messages(mut client: ResMut<MultiplayerClient>) {
    let mut messages = Vec::new();
    if let Some(inbound) = &client.inbound
        && let Ok(inbound) = inbound.lock()
    {
        while let Ok(message) = inbound.try_recv() {
            messages.push(message);
        }
    }

    for message in messages {
        match message {
            ServerMessage::Welcome { player_id } => {
                client.player_id = Some(player_id);
                client.status = "Connected".to_string();
                client.request_lobbies();
            }
            ServerMessage::LobbyList { mut lobbies } => {
                lobbies.sort_by(|left, right| left.name.cmp(&right.name));
                client.lobbies = lobbies;
                client.list_revision += 1;
            }
            ServerMessage::LobbyState { lobby } => {
                client.status = format!("In lobby: {}", lobby.name);
                client.lobby = Some(lobby);
                client.lobby_revision += 1;
            }
            ServerMessage::GameStarted { layout } => {
                client.status = "Game starting...".to_string();
                client.pending_layout = Some(layout);
                client.lobby_revision += 1;
            }
            ServerMessage::PlayerSnapshot { .. }
            | ServerMessage::PlayerJoined { .. }
            | ServerMessage::PlayerLeft { .. } => {}
            ServerMessage::Error { message } => {
                client.status = message;
            }
        }
    }
}

fn sanitize_name(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        "Player".to_string()
    } else {
        trimmed.chars().take(32).collect()
    }
}

fn sanitize_lobby_name(input: &str, display_name: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        format!("{}'s Lobby", sanitize_name(display_name))
    } else {
        trimmed.chars().take(40).collect()
    }
}

fn display_name_path() -> Option<PathBuf> {
    let mut root = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    root.push("GeodeTD");
    Some(root.join("display_name.txt"))
}

fn load_display_name() -> String {
    let Some(path) = display_name_path() else {
        return String::new();
    };
    std::fs::read_to_string(path)
        .map(|name| sanitize_name(&name))
        .unwrap_or_default()
}

fn save_display_name(display_name: &str) -> std::io::Result<()> {
    let Some(path) = display_name_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, display_name)
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_network_thread(
    url: String,
    outbound_rx: Receiver<ClientMessage>,
    inbound_tx: Sender<ServerMessage>,
) {
    std::thread::spawn(move || {
        use std::io::ErrorKind;
        use std::time::Duration;
        use tungstenite::stream::MaybeTlsStream;
        use tungstenite::{Message, connect};

        let Ok((mut socket, _)) = connect(&url) else {
            let _ = inbound_tx.send(ServerMessage::Error {
                message: format!("Could not connect to {url}. Is the server running?"),
            });
            return;
        };

        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(50)));
        }

        loop {
            while let Ok(message) = outbound_rx.try_recv() {
                let Ok(text) = serde_json::to_string(&message) else {
                    continue;
                };
                if socket.write(Message::Text(text.into())).is_err() {
                    let _ = inbound_tx.send(ServerMessage::Error {
                        message: "Connection closed while sending.".to_string(),
                    });
                    return;
                }
            }

            match socket.read() {
                Ok(Message::Text(text)) => {
                    if let Ok(message) = serde_json::from_str::<ServerMessage>(&text) {
                        let _ = inbound_tx.send(message);
                    }
                }
                Ok(Message::Close(_)) => {
                    let _ = inbound_tx.send(ServerMessage::Error {
                        message: "Server connection closed.".to_string(),
                    });
                    return;
                }
                Ok(
                    Message::Ping(_) | Message::Pong(_) | Message::Binary(_) | Message::Frame(_),
                ) => {}
                Err(tungstenite::Error::Io(err))
                    if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(_) => {
                    let _ = inbound_tx.send(ServerMessage::Error {
                        message: "Server connection lost.".to_string(),
                    });
                    return;
                }
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn spawn_network_thread(
    _url: String,
    _outbound_rx: Receiver<ClientMessage>,
    inbound_tx: Sender<ServerMessage>,
) {
    let _ = inbound_tx.send(ServerMessage::Error {
        message: "Multiplayer WebSocket client is not wired for web builds yet.".to_string(),
    });
}
