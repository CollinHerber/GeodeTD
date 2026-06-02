use serde::{Deserialize, Serialize};

pub const MAX_LOBBY_PLAYERS: usize = 8;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum GameMode {
    Standard,
    Random,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LobbyStatus {
    Waiting,
    InGame,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum PhaseSnapshot {
    Build,
    Countdown,
    Wave,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct GridPoint {
    pub col: i32,
    pub row: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SharedBoardLayout {
    pub mode: GameMode,
    pub seed: Option<u64>,
    pub checkpoints: Vec<GridPoint>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbySummary {
    pub id: String,
    pub name: String,
    pub host_player_id: String,
    pub host_name: String,
    pub player_count: usize,
    pub max_players: usize,
    pub status: LobbyStatus,
    pub mode: Option<GameMode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyPlayer {
    pub id: String,
    pub name: String,
    pub is_host: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LobbyState {
    pub id: String,
    pub name: String,
    pub host_player_id: String,
    pub players: Vec<LobbyPlayer>,
    pub max_players: usize,
    pub status: LobbyStatus,
    pub mode: Option<GameMode>,
    pub layout: Option<SharedBoardLayout>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TowerSnapshot {
    pub col: i32,
    pub row: i32,
    pub gem: String,
    pub grade: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WallSnapshot {
    pub col: i32,
    pub row: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BoardSnapshot {
    pub round: u32,
    pub lives: i32,
    pub coins: u32,
    pub phase: PhaseSnapshot,
    pub towers: Vec<TowerSnapshot>,
    pub walls: Vec<WallSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ClientMessage {
    ListLobbies,
    CreateLobby {
        player_name: String,
        lobby_name: String,
    },
    JoinLobby {
        lobby_id: String,
        player_name: String,
    },
    LeaveLobby,
    SetMode {
        mode: GameMode,
    },
    StartGame,
    BoardSnapshot {
        snapshot: BoardSnapshot,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum ServerMessage {
    Welcome {
        player_id: String,
    },
    LobbyList {
        lobbies: Vec<LobbySummary>,
    },
    LobbyState {
        lobby: LobbyState,
    },
    GameStarted {
        layout: SharedBoardLayout,
    },
    PlayerSnapshot {
        player_id: String,
        snapshot: BoardSnapshot,
    },
    PlayerJoined {
        player: LobbyPlayer,
    },
    PlayerLeft {
        player_id: String,
    },
    Error {
        message: String,
    },
}
