use bevy::prelude::*;

use crate::components::{Enemy, GameWorld, Poison, ShotEffect, Slowed, Tower};
use crate::game::{AppScreen, Game, RoundKind};
use crate::gem::GemEffect;

pub fn tower_attack(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut towers: Query<(Entity, &Transform, &mut Tower)>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
    mut poisons: Query<&mut Poison>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());

    // Opal support auras: (tower entity, position, range, cooldown reduction).
    // Snapshotted up front so each tower can be hasted by the others.
    let auras: Vec<(Entity, Vec2, f32, f32)> = towers
        .iter()
        .filter_map(|(entity, transform, tower)| {
            if let GemEffect::Haste { cooldown_reduction } = tower.gem.effect(tower.grade) {
                Some((
                    entity,
                    transform.translation.truncate(),
                    tower.range,
                    cooldown_reduction,
                ))
            } else {
                None
            }
        })
        .collect();

    // Snapshot live enemy positions (and whether they fly) once so targeting and
    // area effects can be computed without holding a mutable borrow of `enemies`.
    let snapshot: Vec<(Entity, Vec2, bool)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.health > 0.0)
        .map(|(entity, transform, enemy)| (entity, transform.translation.truncate(), enemy.flying))
        .collect();

    for (tower_entity, tower_transform, mut tower) in &mut towers {
        let tower_position = tower_transform.translation.truncate();

        // Nearby Opals speed this tower up; their reductions stack, capped.
        let haste = (1.0
            + auras
                .iter()
                .filter(|(entity, position, range, _)| {
                    *entity != tower_entity && position.distance(tower_position) <= *range
                })
                .map(|(_, _, _, reduction)| *reduction)
                .sum::<f32>())
        .min(2.5);

        tower.cooldown.tick(delta.mul_f32(haste));
        if !tower.cooldown.is_finished() {
            continue;
        }

        let target = snapshot
            .iter()
            .filter_map(|(entity, position, flying)| {
                let distance = position.distance(tower_position);
                (distance <= tower.range).then_some((*entity, *position, *flying, distance))
            })
            .min_by(|(_, _, _, left), (_, _, _, right)| left.total_cmp(right))
            .map(|(entity, position, flying, _)| (entity, position, flying));

        let Some((target, target_position, target_flying)) = target else {
            continue;
        };

        tower.cooldown.reset();

        let effect = tower.gem.effect(tower.grade);
        let mut damage = tower.damage * tower.grade.damage_multiplier();

        // Favored gems (Amethyst vs air, Diamond vs ground) hit their preferred
        // enemy class harder.
        if let GemEffect::Favored { air, multiplier } = effect
            && target_flying == air
        {
            damage *= multiplier;
        }

        // Gather secondary damage and the beams to draw for area effects.
        let mut secondary: Vec<(Entity, f32)> = Vec::new();
        let mut beams: Vec<(Vec2, Vec2, Color, f32)> = Vec::new();

        match effect {
            GemEffect::Splash {
                radius,
                damage_fraction,
            } => {
                for (entity, position, _) in &snapshot {
                    if *entity != target && position.distance(target_position) <= radius {
                        secondary.push((*entity, damage * damage_fraction));
                    }
                }
            }
            GemEffect::Multi {
                targets,
                damage_fraction,
            } => {
                // Strike the nearest other in-range enemies, beyond the primary.
                let mut others: Vec<(Entity, Vec2, f32)> = snapshot
                    .iter()
                    .filter(|(entity, _, _)| *entity != target)
                    .filter_map(|(entity, position, _)| {
                        let distance = position.distance(tower_position);
                        (distance <= tower.range).then_some((*entity, *position, distance))
                    })
                    .collect();
                others.sort_by(|(_, _, a), (_, _, b)| a.total_cmp(b));
                for (entity, position, _) in
                    others.into_iter().take(targets.saturating_sub(1) as usize)
                {
                    secondary.push((entity, damage * damage_fraction));
                    beams.push((tower_position, position, tower.gem.color(), 4.0));
                }
            }
            _ => {}
        }

        // Primary beam, drawn last so it sits on top.
        beams.push((tower_position, target_position, tower.gem.color(), 4.0));

        if let Ok((_, _, mut enemy)) = enemies.get_mut(target) {
            enemy.health -= damage;
        }
        for (entity, dmg) in secondary {
            if let Ok((_, _, mut enemy)) = enemies.get_mut(entity) {
                enemy.health -= dmg;
            }
        }

        match effect {
            GemEffect::Slow { factor, duration } => {
                commands.entity(target).insert(Slowed {
                    factor,
                    timer: Timer::from_seconds(duration, TimerMode::Once),
                });
            }
            GemEffect::Poison {
                dps_per_stack,
                duration,
                max_stacks,
                slow_factor,
            } => {
                if let Ok(mut poison) = poisons.get_mut(target) {
                    poison.stacks = (poison.stacks + 1).min(max_stacks);
                    poison.dps_per_stack = dps_per_stack;
                    poison.duration = Timer::from_seconds(duration, TimerMode::Once);
                } else {
                    commands.entity(target).insert(Poison {
                        stacks: 1,
                        dps_per_stack,
                        duration: Timer::from_seconds(duration, TimerMode::Once),
                    });
                }
                // The "slowing" half of the slowing poison.
                commands.entity(target).insert(Slowed {
                    factor: slow_factor,
                    timer: Timer::from_seconds(duration, TimerMode::Once),
                });
            }
            _ => {}
        }

        for (start, end, color, thickness) in beams {
            spawn_beam(&mut commands, start, end, color, thickness);
        }
    }
}

/// Ticks stacking poison damage and removes it when it expires.
pub fn apply_poison(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut enemies: Query<(Entity, &mut Enemy, &mut Poison)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs() * game.speed_multiplier();
    let duration_delta = time.delta().mul_f32(game.speed_multiplier());
    for (entity, mut enemy, mut poison) in &mut enemies {
        poison.duration.tick(duration_delta);
        enemy.health -= poison.stacks as f32 * poison.dps_per_stack * delta;
        if poison.duration.is_finished() {
            commands.entity(entity).remove::<Poison>();
        }
    }
}

/// Expires movement slows so enemies return to full speed.
pub fn update_slow(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut slows: Query<(Entity, &mut Slowed)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (entity, mut slow) in &mut slows {
        slow.timer
            .tick(time.delta().mul_f32(game.speed_multiplier()));
        if slow.timer.is_finished() {
            commands.entity(entity).remove::<Slowed>();
        }
    }
}

/// Despawns enemies killed by any damage source and awards a coin for each.
/// Centralizing this keeps coin rewards correct no matter what landed the kill
/// (direct hit, splash, or poison).
pub fn reap_enemies(
    mut commands: Commands,
    mut game: ResMut<Game>,
    enemies: Query<(Entity, &Enemy)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (entity, enemy) in &enemies {
        if enemy.health <= 0.0 {
            commands.entity(entity).despawn();
            game.coins += 1;
        }
    }
}

pub fn update_enemy_visuals(
    game: Res<Game>,
    mut enemies: Query<(&Enemy, &mut Sprite, Option<&Slowed>, Option<&Poison>)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (enemy, mut sprite, slowed, poisoned) in &mut enemies {
        let health_percent = (enemy.health / enemy.max_health).clamp(0.0, 1.0);
        sprite.color = if slowed.is_some() {
            Color::srgb(0.40, 0.62, 0.95)
        } else if poisoned.is_some() {
            Color::srgb(0.42, 0.74, 0.26)
        } else {
            enemy_tint(enemy.kind, health_percent)
        };
    }
}

/// Base tint per round kind, brightened toward full health so damage still reads
/// as the enemy darkening regardless of its wave type.
fn enemy_tint(kind: RoundKind, health_percent: f32) -> Color {
    match kind {
        RoundKind::Normal => Color::srgb(0.85, 0.12 + 0.42 * health_percent, 0.13),
        RoundKind::Swift => Color::srgb(0.98, 0.48 + 0.34 * health_percent, 0.12),
        RoundKind::Flying => Color::srgb(0.36, 0.62 + 0.28 * health_percent, 0.92),
        RoundKind::Boss => Color::srgb(0.52 + 0.30 * health_percent, 0.10, 0.66),
    }
}

pub fn cleanup_effects(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut effects: Query<(Entity, &mut ShotEffect)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (entity, mut effect) in &mut effects {
        effect
            .timer
            .tick(time.delta().mul_f32(game.speed_multiplier()));
        if effect.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_beam(commands: &mut Commands, start: Vec2, end: Vec2, color: Color, thickness: f32) {
    let delta = end - start;
    let length = delta.length();
    if length <= 1.0 {
        return;
    }

    let midpoint = start + delta * 0.5;
    let angle = delta.y.atan2(delta.x);

    commands.spawn((
        Sprite::from_color(color, Vec2::new(length, thickness)),
        Transform::from_translation(midpoint.extend(20.0))
            .with_rotation(Quat::from_rotation_z(angle)),
        ShotEffect {
            timer: Timer::from_seconds(0.08, TimerMode::Once),
        },
        GameWorld,
    ));
}
