use bevy::prelude::*;

use crate::board::Board;
use crate::components::{Enemy, EnemyAnimation, GameWorld, Slowed};
use crate::enemy_art::EnemyArt;
use crate::game::{AppScreen, Game, Phase, RoundKind, RoundPlan};
use crate::grid::{finish_pos, grid_to_world};

pub fn update_wave_countdown(time: Res<Time>, mut game: ResMut<Game>) {
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Countdown {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());
    game.countdown_timer.tick(delta);
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
    enemy_art: Res<EnemyArt>,
    enemies: Query<Entity, With<Enemy>>,
) {
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Wave {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());
    game.spawn_timer.tick(delta);
    if game.pending_enemies > 0 && game.spawn_timer.just_finished() {
        let plan = RoundPlan::for_round(game.round);
        spawn_enemy(&mut commands, &board.path, &plan, &enemy_art);
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
    mut enemies: Query<(
        Entity,
        &mut Transform,
        &mut Sprite,
        &mut Enemy,
        Option<&Slowed>,
    )>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs() * game.speed_multiplier();
    let flight_target = grid_to_world(finish_pos());

    for (entity, mut transform, mut sprite, mut enemy, slowed) in &mut enemies {
        // Flying enemies bee-line for the finish; grounded enemies that ran out of
        // path have reached the end. Either way, leaking costs a life.
        let reached_end = if enemy.flying {
            false
        } else {
            enemy.next_path_index >= board.path.len()
        };
        if reached_end {
            game.lives -= 1;
            commands.entity(entity).despawn();
            continue;
        }

        let speed = enemy.speed * slowed.map_or(1.0, |slow| slow.factor);
        let target = if enemy.flying {
            flight_target
        } else {
            grid_to_world(board.path[enemy.next_path_index])
        };
        let current = transform.translation.truncate();
        let to_target = target - current;
        let step = speed * delta;
        if to_target.x.abs() > 0.1 {
            sprite.flip_x = to_target.x < 0.0;
        }

        if to_target.length() <= step {
            if enemy.flying {
                // Flew off the far edge; that's a leak.
                game.lives -= 1;
                commands.entity(entity).despawn();
            } else {
                transform.translation = target.extend(10.0);
                enemy.next_path_index += 1;
            }
        } else {
            transform.translation += (to_target.normalize() * step).extend(0.0);
        }
    }
}

pub fn animate_enemies(
    time: Res<Time>,
    game: Res<Game>,
    mut enemies: Query<(&mut Sprite, &mut EnemyAnimation)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());
    for (mut sprite, mut animation) in &mut enemies {
        animation.timer.tick(delta);
        if !animation.timer.just_finished() {
            continue;
        }

        let Some(atlas) = sprite.texture_atlas.as_mut() else {
            continue;
        };

        atlas.index = if atlas.index >= animation.last {
            animation.first
        } else {
            atlas.index + 1
        };
    }
}

fn spawn_enemy(
    commands: &mut Commands,
    path: &[crate::grid::GridPos],
    plan: &RoundPlan,
    enemy_art: &EnemyArt,
) {
    if path.len() < 2 {
        return;
    }

    // Bosses are visibly larger; swift/flying enemies are a touch smaller.
    let size = match plan.kind {
        RoundKind::Boss => 46.0,
        RoundKind::Swift => 16.0,
        RoundKind::Flying => 17.0,
        RoundKind::Normal => 19.0,
    } * 1.6;

    // Flying enemies start a layer higher so they read as airborne above towers.
    let z = if plan.flying { 11.0 } else { 10.0 };
    let mut sprite = Sprite::from_atlas_image(
        enemy_art.gemling_image.clone(),
        TextureAtlas {
            layout: enemy_art.gemling_layout.clone(),
            index: 0,
        },
    );
    sprite.custom_size = Some(Vec2::splat(size));
    sprite.color = plan.kind.accent();

    commands.spawn((
        sprite,
        Transform::from_translation(grid_to_world(path[0]).extend(z)),
        Enemy {
            next_path_index: 1,
            health: plan.health,
            max_health: plan.health,
            speed: plan.speed,
            kind: plan.kind,
            flying: plan.flying,
        },
        EnemyAnimation {
            first: 0,
            last: 7,
            timer: Timer::from_seconds(0.09, TimerMode::Repeating),
        },
        GameWorld,
    ));
}
