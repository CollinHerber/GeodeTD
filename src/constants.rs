pub const PLAY_AREA_SCALE: i32 = 4;
pub const BASE_GRID_COLUMNS: i32 = 29;
pub const BASE_GRID_ROWS: i32 = 17;
pub const GRID_COLUMNS: i32 = BASE_GRID_COLUMNS * PLAY_AREA_SCALE;
pub const GRID_ROWS: i32 = BASE_GRID_ROWS * PLAY_AREA_SCALE;
pub const CELL_SIZE: f32 = 40.0;
pub const OFFER_COUNT: usize = 5;
pub const WAVE_COUNTDOWN_SECONDS: f32 = 3.0;

/// Gold cost of the first "upgrading chances" purchase; each later one costs
/// `CHANCE_COST_STEP` more than the previous (escalating tree).
pub const CHANCE_BASE_COST: u32 = 10;
pub const CHANCE_COST_STEP: u32 = 5;
