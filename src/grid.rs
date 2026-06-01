use bevy::prelude::*;

use crate::constants::{
    BASE_GRID_COLUMNS, BASE_GRID_ROWS, CELL_SIZE, GRID_COLUMNS, GRID_ROWS, PLAY_AREA_SCALE,
};

const BOARD_Y_OFFSET: f32 = -8.0;

/// Cardinal steps (col, row). Used for orthogonal movement and as the gate cells
/// that guard diagonal moves (see `DIAGONAL_STEPS`).
pub const ORTHOGONAL_STEPS: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

/// Diagonal steps (col, row). Enemies may move along these so they can cut across
/// the board instead of staircasing, but only when both orthogonal cells they pass
/// between are open (no squeezing through a diagonal gap — see `board::moves`).
pub const DIAGONAL_STEPS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

// Integer movement costs (scaled by 10) so A* can order nodes without floats.
// 14 ≈ 10 * sqrt(2), the diagonal length relative to a unit cardinal step.
pub const ORTHOGONAL_COST: i32 = 10;
pub const DIAGONAL_COST: i32 = 14;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridPos {
    pub col: i32,
    pub row: i32,
}

impl GridPos {
    pub fn new(col: i32, row: i32) -> Self {
        Self { col, row }
    }

    pub fn offset(self, delta: (i32, i32)) -> GridPos {
        GridPos::new(self.col + delta.0, self.row + delta.1)
    }

    pub fn in_bounds(self) -> bool {
        self.col >= 0 && self.col < GRID_COLUMNS && self.row >= 0 && self.row < GRID_ROWS
    }
}

pub fn start_pos() -> GridPos {
    scaled_grid_pos(1, BASE_GRID_ROWS / 2)
}

pub fn finish_pos() -> GridPos {
    scaled_grid_pos(BASE_GRID_COLUMNS - 2, BASE_GRID_ROWS / 2)
}

pub fn scaled_grid_pos(col: i32, row: i32) -> GridPos {
    GridPos::new(col * PLAY_AREA_SCALE, row * PLAY_AREA_SCALE)
}

pub fn board_width() -> f32 {
    GRID_COLUMNS as f32 * CELL_SIZE
}

pub fn board_height() -> f32 {
    GRID_ROWS as f32 * CELL_SIZE
}

pub fn grid_to_world(pos: GridPos) -> Vec2 {
    Vec2::new(
        -board_width() * 0.5 + pos.col as f32 * CELL_SIZE + CELL_SIZE * 0.5,
        -board_height() * 0.5 + pos.row as f32 * CELL_SIZE + CELL_SIZE * 0.5 + BOARD_Y_OFFSET,
    )
}

pub fn world_to_grid(world: Vec2) -> Option<GridPos> {
    let col = ((world.x + board_width() * 0.5) / CELL_SIZE).floor() as i32;
    let row = ((world.y - BOARD_Y_OFFSET + board_height() * 0.5) / CELL_SIZE).floor() as i32;
    let pos = GridPos::new(col, row);
    pos.in_bounds().then_some(pos)
}
