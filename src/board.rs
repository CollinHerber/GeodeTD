use bevy::prelude::*;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::{GRID_COLUMNS, GRID_ROWS};
use crate::game::GameMode;
use crate::grid::{
    DIAGONAL_COST, DIAGONAL_STEPS, GridPos, ORTHOGONAL_COST, ORTHOGONAL_STEPS, finish_pos,
    start_pos,
};
use crate::rng::OfferRng;

pub const CHECKPOINT_COUNT: usize = 4;

#[derive(Resource)]
pub struct Board {
    pub towers: HashMap<GridPos, Entity>,
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
            path,
            checkpoints,
        }
    }

    pub fn reset_for_mode(&mut self, mode: GameMode) {
        *self = Board::for_mode(mode);
    }

    pub fn occupied_cells(&self) -> HashSet<GridPos> {
        self.towers.keys().copied().collect()
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
        if complete.is_empty() {
            complete.extend(segment_path);
        } else {
            complete.extend(segment_path.into_iter().skip(1));
        }
    }

    Some(complete)
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

    while checkpoints.len() < CHECKPOINT_COUNT {
        let col = 4 + rng.next_index((GRID_COLUMNS - 8) as usize) as i32;
        let row = 2 + rng.next_index((GRID_ROWS - 4) as usize) as i32;
        let pos = GridPos::new(col, row);

        if protected.contains(&pos) || checkpoints.contains(&pos) {
            continue;
        }

        checkpoints.push(pos);
    }

    checkpoints
}

fn standard_checkpoints() -> Vec<GridPos> {
    let center = GridPos::new(GRID_COLUMNS / 2, GRID_ROWS / 2);

    vec![
        center,
        GridPos::new(6, 3),
        GridPos::new(22, 13),
        GridPos::new(6, 13),
        GridPos::new(22, 3),
    ]
}
