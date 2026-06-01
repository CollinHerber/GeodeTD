use bevy::prelude::*;
use std::collections::HashSet;

use crate::components::{Enemy, GameWorld, Poison, ShotEffect, Slowed, Tower};
use crate::game::{AppScreen, Game, RoundKind};
use crate::gem::GemEffect;

/// How far chain lightning can arc between successive enemies.
const CHAIN_JUMP_RADIUS: f32 = 96.0;
const LIGHTNING_COLOR: Color = Color::srgb(0.62, 0.86, 1.0);

pub fn tower_attack(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut towers: Query<(&Transform, &mut Tower)>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
    mut poisons: Query<&mut Poison>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());

    // Snapshot live enemy positions once so targeting and area/chain effects can be
    // computed without holding a mutable borrow of `enemies`.
    let snapshot: Vec<(Entity, Vec2)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.health > 0.0)
        .map(|(entity, transform, _)| (entity, transform.translation.truncate()))
        .collect();

    for (tower_transform, mut tower) in &mut towers {
        tower.cooldown.tick(delta);
        if !tower.cooldown.is_finished() {
            continue;
        }

        let tower_position = tower_transform.translation.truncate();
        let target = snapshot
            .iter()
            .filter_map(|(entity, position)| {
                let distance = position.distance(tower_position);
                (distance <= tower.range).then_some((*entity, *position, distance))
            })
            .min_by(|(_, _, left), (_, _, right)| left.total_cmp(right))
            .map(|(entity, position, _)| (entity, position));

        let Some((target, target_position)) = target else {
            continue;
        };

        tower.cooldown.reset();

        let effect = tower.gem.effect(tower.grade);
        let mut damage = tower.damage * tower.grade.damage_multiplier();

        let mut crit = false;
        if let GemEffect::Crit { chance, multiplier } = effect
            && game.rng.next_f32() < chance
        {
            damage *= multiplier;
            crit = true;
        }

        // Gather secondary damage and the beams to draw for area/chain effects.
        let mut secondary: Vec<(Entity, f32)> = Vec::new();
        let mut beams: Vec<(Vec2, Vec2, Color, f32)> = Vec::new();

        match effect {
            GemEffect::Splash {
                radius,
                damage_fraction,
            } => {
                for (entity, position) in &snapshot {
                    if *entity != target && position.distance(target_position) <= radius {
                        secondary.push((*entity, damage * damage_fraction));
                    }
                }
            }
            GemEffect::Chain {
                chance,
                jumps,
                damage_fraction,
            } if game.rng.next_f32() < chance => {
                let mut from = target_position;
                let mut hit: HashSet<Entity> = HashSet::from([target]);
                for _ in 0..jumps {
                    let next = snapshot
                        .iter()
                        .filter(|(entity, position)| {
                            !hit.contains(entity) && position.distance(from) <= CHAIN_JUMP_RADIUS
                        })
                        .min_by(|(_, a), (_, b)| a.distance(from).total_cmp(&b.distance(from)));
                    let Some((entity, position)) = next else {
                        break;
                    };
                    secondary.push((*entity, damage * damage_fraction));
                    beams.push((from, *position, LIGHTNING_COLOR, 5.0));
                    from = *position;
                    hit.insert(*entity);
                }
            }
            _ => {}
        }

        // Primary beam (drawn last so it sits on top), thicker/white on a crit.
        let (primary_color, primary_thickness) = if crit {
            (Color::srgb(1.0, 0.97, 0.85), 7.0)
        } else {
            (tower.gem.color(), 4.0)
        };
        beams.push((
            tower_position,
            target_position,
            primary_color,
            primary_thickness,
        ));

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
