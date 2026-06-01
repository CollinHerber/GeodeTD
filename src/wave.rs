use bevy::prelude::*;

use crate::board::Board;
use crate::components::{Enemy, GameWorld, Slowed};
use crate::game::{AppScreen, Game, Phase};
use crate::grid::grid_to_world;

pub fn update_wave_countdown(time: Res<Time>, mut game: ResMut<Game>) {
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Countdown {
        return;
    }

    game.countdown_timer.tick(time.delta());
    game.message = format!(
        "Wave starts in {:.1} seconds.",
        game.countdown_timer.remaining_secs().max(0.0)
    );

    if game.countdown_timer.is_finished() {
        game.begin_wave();
    }
}

pub fn run_wave(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    board: Res<Board>,
    enemies: Query<Entity, With<Enemy>>,
) {
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Wave {
        return;
    }

    game.spawn_timer.tick(time.delta());
    if game.pending_enemies > 0 && game.spawn_timer.just_finished() {
        spawn_enemy(&mut commands, &board.path, game.round);
        game.pending_enemies -= 1;
    }

    if game.pending_enemies == 0 && enemies.iter().next().is_none() {
        game.begin_build_round();
    }
}

pub fn move_enemies(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    board: Res<Board>,
    mut enemies: Query<(Entity, &mut Transform, &mut Enemy, Option<&Slowed>)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs();

    for (entity, mut transform, mut enemy, slowed) in &mut enemies {
        if enemy.next_path_index >= board.path.len() {
            game.lives -= 1;
            commands.entity(entity).despawn();
            continue;
        }

        let speed = enemy.speed * slowed.map_or(1.0, |slow| slow.factor);
        let target = grid_to_world(board.path[enemy.next_path_index]);
        let current = transform.translation.truncate();
        let to_target = target - current;
        let step = speed * delta;

        if to_target.length() <= step {
            transform.translation = target.extend(10.0);
            enemy.next_path_index += 1;
        } else {
            transform.translation += (to_target.normalize() * step).extend(0.0);
        }
    }
}

fn spawn_enemy(commands: &mut Commands, path: &[crate::grid::GridPos], round: u32) {
    if path.len() < 2 {
        return;
    }

    let max_health = 30.0 + round as f32 * 8.0;
    commands.spawn((
        Sprite::from_color(Color::srgb(0.85, 0.18, 0.16), Vec2::splat(19.0)),
        Transform::from_translation(grid_to_world(path[0]).extend(10.0)),
        Enemy {
            next_path_index: 1,
            health: max_health,
            max_health,
            speed: 72.0 + round as f32 * 3.0,
        },
        GameWorld,
    ));
}
