use bevy::prelude::*;
use geode_td_shared::SharedBoardLayout;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{
    BASE_GRID_COLUMNS, BASE_GRID_ROWS, GRID_COLUMNS, GRID_ROWS, PLAY_AREA_SCALE,
};
use crate::game::GameMode;
use crate::grid::{
    DIAGONAL_COST, DIAGONAL_STEPS, GridPos, ORTHOGONAL_COST, ORTHOGONAL_STEPS, finish_pos,
    scaled_grid_pos, start_pos,
};
use crate::rng::OfferRng;

pub const CHECKPOINT_COUNT: usize = 4;

#[derive(Resource)]
pub struct Board {
    pub towers: HashMap<GridPos, Entity>,
    pub walls: HashSet<GridPos>,
    pub path: Vec<GridPos>,
    pub checkpoints: Vec<GridPos>,
}

impl Board {
    pub fn new() -> Self {
        Self::for_mode(GameMode::Standard)
    }

    pub fn for_mode(mode: GameMode) -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_nanos() as u64)
            .unwrap_or(0xB0A4D);
        let mut rng = OfferRng::new(seed);
        let checkpoints = match mode {
            GameMode::Standard => standard_checkpoints(),
            GameMode::Random => random_checkpoints(&mut rng),
        };
        let path = find_complete_path(&HashSet::new(), &checkpoints)
            .expect("empty board should always have a checkpoint path");

        Self {
            towers: HashMap::new(),
            walls: HashSet::new(),
            path,
            checkpoints,
        }
    }

    pub fn reset_for_mode(&mut self, mode: GameMode) {
        *self = Board::for_mode(mode);
    }

    pub fn reset_for_shared_layout(&mut self, layout: &SharedBoardLayout) {
        let checkpoints = layout
            .checkpoints
            .iter()
            .map(|point| GridPos::new(point.col, point.row))
            .collect::<Vec<_>>();
        let path = find_complete_path(&HashSet::new(), &checkpoints)
            .expect("server layout should always have an empty-board path");

        *self = Self {
            towers: HashMap::new(),
            walls: HashSet::new(),
            path,
            checkpoints,
        };
    }

    pub fn occupied_cells(&self) -> HashSet<GridPos> {
        let mut occupied = self.walls.clone();
        occupied.extend(self.towers.keys().copied());
        occupied
    }

    pub fn protected_cells(&self) -> HashSet<GridPos> {
        let mut protected = HashSet::from([start_pos(), finish_pos()]);
        protected.extend(self.checkpoints.iter().copied());
        protected
    }

    pub fn recalculate_path(&mut self) -> bool {
        if let Some(path) = find_complete_path(&self.occupied_cells(), &self.checkpoints) {
            self.path = path;
            true
        } else {
            false
        }
    }
}

pub fn find_complete_path(
    blocked: &HashSet<GridPos>,
    checkpoints: &[GridPos],
) -> Option<Vec<GridPos>> {
    let mut targets = Vec::with_capacity(checkpoints.len() + 2);
    targets.push(start_pos());
    targets.extend_from_slice(checkpoints);
    targets.push(finish_pos());

    let mut complete = Vec::new();

    for segment in targets.windows(2) {
        let segment_path = find_path_between(segment[0], segment[1], blocked)?;
        // Collapse the grid path into any-angle straight runs so enemies travel at
        // the true shortest heading instead of staircasing along grid directions.
        let segment_path = smooth_path(&segment_path, blocked);
        if complete.is_empty() {
            complete.extend(segment_path);
        } else {
            complete.extend(segment_path.into_iter().skip(1));
        }
    }

    Some(complete)
}

/// String-pulls a grid path into the fewest waypoints that still avoid obstacles:
/// a waypoint is kept only when the previous anchor loses line of sight to the
/// next point. Segment endpoints (start/checkpoint/finish) are always preserved.
fn smooth_path(path: &[GridPos], blocked: &HashSet<GridPos>) -> Vec<GridPos> {
    if path.len() <= 2 {
        return path.to_vec();
    }

    let mut result = vec![path[0]];
    let mut anchor = path[0];
    for index in 1..path.len() - 1 {
        if !line_of_sight(anchor, path[index + 1], blocked) {
            result.push(path[index]);
            anchor = path[index];
        }
    }
    result.push(path[path.len() - 1]);
    result
}

/// Whether the straight segment between two cell centers stays entirely on open
/// ground. Uses a supercover walk so every cell the line touches is checked, and
/// treats an exact corner crossing like a diagonal move: both flanking cells must
/// be open (no squeezing through a diagonal gap), matching `moves`.
fn line_of_sight(a: GridPos, b: GridPos, blocked: &HashSet<GridPos>) -> bool {
    if blocked.contains(&a) {
        return false;
    }

    let mut col = a.col;
    let mut row = a.row;
    let steps_x = (b.col - a.col).abs();
    let steps_y = (b.row - a.row).abs();
    let sign_x = (b.col - a.col).signum();
    let sign_y = (b.row - a.row).signum();

    let (mut done_x, mut done_y) = (0, 0);
    while done_x < steps_x || done_y < steps_y {
        let compare = (1 + 2 * done_x) * steps_y - (1 + 2 * done_y) * steps_x;
        if compare == 0 {
            // The line crosses a grid corner; refuse if either flank is blocked.
            if blocked.contains(&GridPos::new(col + sign_x, row))
                || blocked.contains(&GridPos::new(col, row + sign_y))
            {
                return false;
            }
            col += sign_x;
            row += sign_y;
            done_x += 1;
            done_y += 1;
        } else if compare < 0 {
            col += sign_x;
            done_x += 1;
        } else {
            row += sign_y;
            done_y += 1;
        }

        if blocked.contains(&GridPos::new(col, row)) {
            return false;
        }
    }

    true
}

/// A* over an 8-connected grid. Diagonal moves are allowed so enemies can cut
/// across open space instead of staircasing, which is what lets them travel through
/// the middle of the standard layout. Uses the octile heuristic and integer costs.
fn find_path_between(
    start: GridPos,
    finish: GridPos,
    blocked: &HashSet<GridPos>,
) -> Option<Vec<GridPos>> {
    let mut came_from: HashMap<GridPos, GridPos> = HashMap::new();
    let mut cost_so_far: HashMap<GridPos, i32> = HashMap::from([(start, 0)]);
    let mut frontier: BinaryHeap<Reverse<(i32, GridPos)>> = BinaryHeap::new();
    frontier.push(Reverse((heuristic(start, finish), start)));

    while let Some(Reverse((_, current))) = frontier.pop() {
        if current == finish {
            let mut path = vec![current];
            let mut step = current;

            while step != start {
                step = came_from[&step];
                path.push(step);
            }

            path.reverse();
            return Some(path);
        }

        let current_cost = cost_so_far[&current];
        for (neighbor, step_cost) in moves(current, blocked) {
            let new_cost = current_cost + step_cost;
            if new_cost < *cost_so_far.get(&neighbor).unwrap_or(&i32::MAX) {
                cost_so_far.insert(neighbor, new_cost);
                came_from.insert(neighbor, current);
                let priority = new_cost + heuristic(neighbor, finish);
                frontier.push(Reverse((priority, neighbor)));
            }
        }
    }

    None
}

/// Open neighbors of `pos` with their step cost. Diagonals are only offered when
/// both shared orthogonal cells are open, so a diagonal pair of towers still walls
/// the route off instead of letting enemies slip through the corner.
fn moves(pos: GridPos, blocked: &HashSet<GridPos>) -> Vec<(GridPos, i32)> {
    let is_open = |cell: GridPos| cell.in_bounds() && !blocked.contains(&cell);

    let mut result = Vec::with_capacity(8);

    for step in ORTHOGONAL_STEPS {
        let neighbor = pos.offset(step);
        if is_open(neighbor) {
            result.push((neighbor, ORTHOGONAL_COST));
        }
    }

    for step in DIAGONAL_STEPS {
        let neighbor = pos.offset(step);
        let side_a = pos.offset((step.0, 0));
        let side_b = pos.offset((0, step.1));
        if is_open(neighbor) && is_open(side_a) && is_open(side_b) {
            result.push((neighbor, DIAGONAL_COST));
        }
    }

    result
}

/// Octile distance: the exact shortest 8-connected cost across open ground, so it
/// stays admissible and A* returns a true shortest path.
fn heuristic(from: GridPos, to: GridPos) -> i32 {
    let dx = (from.col - to.col).abs();
    let dy = (from.row - to.row).abs();
    ORTHOGONAL_COST * (dx - dy).abs() + DIAGONAL_COST * dx.min(dy)
}

fn random_checkpoints(rng: &mut OfferRng) -> Vec<GridPos> {
    let mut checkpoints = Vec::with_capacity(CHECKPOINT_COUNT);
    let protected = HashSet::from([start_pos(), finish_pos()]);
    let col_margin = 4 * PLAY_AREA_SCALE;
    let row_margin = 2 * PLAY_AREA_SCALE;

    while checkpoints.len() < CHECKPOINT_COUNT {
        let col = col_margin + rng.next_index((GRID_COLUMNS - col_margin * 2) as usize) as i32;
        let row = row_margin + rng.next_index((GRID_ROWS - row_margin * 2) as usize) as i32;
        let pos = GridPos::new(col, row);

        if protected.contains(&pos) || checkpoints.contains(&pos) {
            continue;
        }

        checkpoints.push(pos);
    }

    checkpoints
}

fn standard_checkpoints() -> Vec<GridPos> {
    let center = scaled_grid_pos(BASE_GRID_COLUMNS / 2, BASE_GRID_ROWS / 2);

    vec![
        center,
        scaled_grid_pos(6, 3),
        scaled_grid_pos(22, 13),
        scaled_grid_pos(6, 13),
        scaled_grid_pos(22, 3),
    ]
}
