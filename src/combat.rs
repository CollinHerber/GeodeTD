use bevy::prelude::*;

use crate::components::{Enemy, GameWorld, ShotEffect, Tower};
use crate::game::{AppScreen, Game};

pub fn tower_attack(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut towers: Query<(&Transform, &mut Tower)>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (tower_transform, mut tower) in &mut towers {
        tower.cooldown.tick(time.delta());
        if !tower.cooldown.is_finished() {
            continue;
        }

        let tower_position = tower_transform.translation.truncate();
        let target = enemies
            .iter()
            .filter_map(|(entity, enemy_transform, enemy)| {
                let distance = enemy_transform
                    .translation
                    .truncate()
                    .distance(tower_position);

                (enemy.health > 0.0 && distance <= tower.range).then_some((entity, distance))
            })
            .min_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(entity, _)| entity);

        let Some(target) = target else {
            continue;
        };

        if let Ok((_, enemy_transform, mut enemy)) = enemies.get_mut(target) {
            enemy.health -= tower.damage * tower.grade.damage_multiplier();
            tower.cooldown.reset();
            spawn_shot_effect(
                &mut commands,
                tower_position,
                enemy_transform.translation.truncate(),
                tower.gem.color(),
            );
            if enemy.health <= 0.0 {
                commands.entity(target).despawn();
                game.coins += 1;
            }
        }
    }
}

pub fn update_enemy_visuals(game: Res<Game>, mut enemies: Query<(&Enemy, &mut Sprite)>) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (enemy, mut sprite) in &mut enemies {
        let health_percent = (enemy.health / enemy.max_health).clamp(0.0, 1.0);
        sprite.color = Color::srgb(0.85, 0.12 + 0.42 * health_percent, 0.13);
    }
}

pub fn cleanup_effects(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut effects: Query<(Entity, &mut ShotEffect)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (entity, mut effect) in &mut effects {
        effect.timer.tick(time.delta());
        if effect.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_shot_effect(commands: &mut Commands, start: Vec2, end: Vec2, color: Color) {
    let delta = end - start;
    let length = delta.length();
    if length <= 1.0 {
        return;
    }

    let midpoint = start + delta * 0.5;
    let angle = delta.y.atan2(delta.x);

    commands.spawn((
        Sprite::from_color(color, Vec2::new(length, 4.0)),
        Transform::from_translation(midpoint.extend(20.0))
            .with_rotation(Quat::from_rotation_z(angle)),
        ShotEffect {
            timer: Timer::from_seconds(0.08, TimerMode::Once),
        },
        GameWorld,
    ));
}
