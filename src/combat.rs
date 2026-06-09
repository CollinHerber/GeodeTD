use bevy::prelude::*;

use crate::components::{
    ArmorBroken, Burning, Enemy, GameWorld, Poison, ShotEffect, Slowed, Stunned, Tower, VfxFade,
    armor_reduction_damage_multiplier,
};
use crate::game::{AppScreen, Game, RoundKind};
use crate::gem::GemEffect;

#[allow(clippy::too_many_arguments)]
pub fn tower_attack(
    mut commands: Commands,
    time: Res<Time>,
    mut game: ResMut<Game>,
    mut towers: Query<(Entity, &Transform, &mut Tower)>,
    mut enemies: Query<(Entity, &Transform, &mut Enemy)>,
    mut poisons: Query<&mut Poison>,
    mut burns: Query<&mut Burning>,
    mut armor_breaks: Query<&mut ArmorBroken>,
    mut stuns: Query<&mut Stunned>,
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
            if let GemEffect::Haste { cooldown_reduction } = tower.effect() {
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
    let air_armor_auras: Vec<(Vec2, f32, f32)> = towers
        .iter()
        .filter_map(|(_, transform, tower)| {
            if let GemEffect::AirArmorAura { armor_reduction } = tower.effect() {
                Some((
                    transform.translation.truncate(),
                    tower.range,
                    armor_reduction,
                ))
            } else {
                None
            }
        })
        .collect();
    let ground_armor_auras: Vec<(Vec2, f32, f32)> = towers
        .iter()
        .filter_map(|(_, transform, tower)| {
            if let GemEffect::Tourmaline {
                armor_reduction,
                armor_range,
                ..
            } = tower.effect()
            {
                Some((
                    transform.translation.truncate(),
                    armor_range,
                    armor_reduction,
                ))
            } else {
                None
            }
        })
        .collect();

    // Snapshot live enemy positions (and whether they fly) once so targeting and
    // area effects can be computed without holding a mutable borrow of `enemies`.
    let snapshot: Vec<(Entity, Vec2, bool, f32)> = enemies
        .iter()
        .filter(|(_, _, enemy)| enemy.health > 0.0)
        .map(|(entity, transform, enemy)| {
            let position = transform.translation.truncate();
            let armor_multiplier = armor_breaks
                .get(entity)
                .map_or(1.0, |armor| armor.damage_multiplier())
                * air_armor_multiplier(position, enemy.flying, &air_armor_auras)
                * ground_armor_multiplier(position, enemy.flying, &ground_armor_auras);
            (entity, position, enemy.flying, armor_multiplier)
        })
        .collect();

    for (tower_entity, tower_transform, mut tower) in &mut towers {
        let tower_position = tower_transform.translation.truncate();
        let effect = tower.effect();
        if !effect.uses_direct_attack() {
            continue;
        }

        // Nearby Opals speed this tower up; their reductions stack, capped.
        let haste = if effect.ignores_opal_effects() {
            1.0
        } else {
            (1.0 + auras
                .iter()
                .filter(|(entity, position, range, _)| {
                    *entity != tower_entity && position.distance(tower_position) <= *range
                })
                .map(|(_, _, _, reduction)| *reduction)
                .sum::<f32>())
            .min(2.5)
        };

        tower.cooldown.tick(delta.mul_f32(haste));
        if !tower.cooldown.is_finished() {
            continue;
        }

        let target = snapshot
            .iter()
            .filter_map(|(entity, position, flying, armor_multiplier)| {
                if !effect.can_target(*flying) {
                    return None;
                }
                let distance = position.distance(tower_position);
                (distance <= tower.range).then_some((
                    *entity,
                    *position,
                    *flying,
                    *armor_multiplier,
                    distance,
                ))
            })
            .min_by(|(_, _, _, _, left), (_, _, _, _, right)| left.total_cmp(right))
            .map(|(entity, position, flying, armor_multiplier, _)| {
                (entity, position, flying, armor_multiplier)
            });

        let Some((target, target_position, target_flying, target_armor_multiplier)) = target else {
            continue;
        };

        tower.cooldown.reset();

        let mut damage = tower.attack_damage();

        if let GemEffect::AncientBlood {
            triple_chance,
            triple_multiplier,
            ..
        } = effect
            && roll_chance(&mut game.rng, triple_chance)
        {
            damage *= triple_multiplier;
        }
        if let GemEffect::ArmorBreak {
            crit_chance,
            crit_multiplier,
            ..
        } = effect
            && roll_chance(&mut game.rng, crit_chance)
        {
            damage *= crit_multiplier;
        }
        if let GemEffect::JadePoison {
            crit_chance,
            crit_multiplier,
            ..
        } = effect
            && roll_chance(&mut game.rng, crit_chance)
        {
            damage *= crit_multiplier;
        }
        if let GemEffect::Critical {
            chance, multiplier, ..
        } = effect
            && roll_chance(&mut game.rng, chance)
        {
            damage *= multiplier;
        }

        // Favored gems (Amethyst vs air, Diamond vs ground) hit their preferred
        // enemy class harder.
        if let GemEffect::Favored { air, multiplier } = effect
            && target_flying == air
        {
            damage *= multiplier;
        }

        // Gather secondary damage and the beams to draw for area effects.
        let mut secondary: Vec<(Entity, f32)> = Vec::new();
        let mut beams: Vec<(Vec2, Vec2, [f32; 3], f32)> = Vec::new();
        let mut splash_slow: Vec<Entity> = Vec::new();

        match effect {
            GemEffect::Splash {
                radius,
                damage_fraction,
            } => {
                for (entity, position, _, armor_multiplier) in &snapshot {
                    if *entity != target && position.distance(target_position) <= radius {
                        secondary.push((*entity, damage * damage_fraction * *armor_multiplier));
                    }
                }
            }
            GemEffect::SlowSplash {
                radius,
                damage_fraction,
                ..
            }
            | GemEffect::SlowSplashBoost {
                radius,
                damage_fraction,
                ..
            } => {
                splash_slow.push(target);
                for (entity, position, _, armor_multiplier) in &snapshot {
                    if *entity != target && position.distance(target_position) <= radius {
                        secondary.push((*entity, damage * damage_fraction * *armor_multiplier));
                        splash_slow.push(*entity);
                    }
                }
            }
            GemEffect::Multi {
                targets,
                damage_fraction,
            } => {
                // Strike the nearest other in-range enemies, beyond the primary.
                let mut others: Vec<(Entity, Vec2, f32, f32)> = snapshot
                    .iter()
                    .filter(|(entity, _, _, _)| *entity != target)
                    .filter_map(|(entity, position, _, armor_multiplier)| {
                        let distance = position.distance(tower_position);
                        (distance <= tower.range).then_some((
                            *entity,
                            *position,
                            *armor_multiplier,
                            distance,
                        ))
                    })
                    .collect();
                others.sort_by(|(_, _, _, a), (_, _, _, b)| a.total_cmp(b));
                for (entity, position, armor_multiplier, _) in
                    others.into_iter().take(targets.saturating_sub(1) as usize)
                {
                    secondary.push((entity, damage * damage_fraction * armor_multiplier));
                    beams.push((tower_position, position, tower.srgb(), 4.0));
                }
            }
            GemEffect::Area { damage_fraction } => {
                for (entity, position, _, armor_multiplier) in &snapshot {
                    if *entity != target && position.distance(tower_position) <= tower.range {
                        secondary.push((*entity, damage * damage_fraction * *armor_multiplier));
                        beams.push((tower_position, *position, tower.srgb(), 3.2));
                    }
                }
            }
            _ => {}
        }

        // Primary beam, drawn last so it sits on top.
        beams.push((tower_position, target_position, tower.srgb(), 4.0));

        if let Ok((_, _, mut enemy)) = enemies.get_mut(target) {
            enemy.health -= damage * target_armor_multiplier;
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
            GemEffect::SlowSplash {
                slow_factor,
                slow_duration,
                ..
            }
            | GemEffect::SlowSplashBoost {
                slow_factor,
                slow_duration,
                ..
            } => {
                for entity in splash_slow {
                    commands.entity(entity).insert(Slowed {
                        factor: slow_factor,
                        timer: Timer::from_seconds(slow_duration, TimerMode::Once),
                    });
                }
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
            GemEffect::AncientBlood {
                burn_chance,
                burn_dps,
                burn_duration,
                ..
            } if roll_chance(&mut game.rng, burn_chance) => {
                if let Ok(mut burn) = burns.get_mut(target) {
                    burn.dps = burn_dps;
                    burn.duration = Timer::from_seconds(burn_duration, TimerMode::Once);
                } else {
                    commands.entity(target).insert(Burning {
                        dps: burn_dps,
                        duration: Timer::from_seconds(burn_duration, TimerMode::Once),
                    });
                }
            }
            GemEffect::AncientBlood {
                burn_chance: _,
                burn_dps: _,
                burn_duration: _,
                ..
            } => {}
            GemEffect::Tourmaline {
                proc_chance,
                proc_total_damage,
                proc_duration,
                ..
            } if roll_chance(&mut game.rng, proc_chance) => {
                let burn_dps = proc_total_damage / proc_duration;
                if let Ok(mut burn) = burns.get_mut(target) {
                    burn.dps = burn.dps.max(burn_dps);
                    burn.duration = Timer::from_seconds(proc_duration, TimerMode::Once);
                } else {
                    commands.entity(target).insert(Burning {
                        dps: burn_dps,
                        duration: Timer::from_seconds(proc_duration, TimerMode::Once),
                    });
                }
            }
            GemEffect::Tourmaline { .. } => {}
            GemEffect::ArmorBreak {
                armor_reduction,
                duration,
                ..
            } => {
                if let Ok(mut armor_break) = armor_breaks.get_mut(target) {
                    armor_break.reduction = armor_break.reduction.max(armor_reduction);
                    armor_break.duration = Timer::from_seconds(duration, TimerMode::Once);
                } else {
                    commands.entity(target).insert(ArmorBroken {
                        reduction: armor_reduction,
                        duration: Timer::from_seconds(duration, TimerMode::Once),
                    });
                }
            }
            GemEffect::JadePoison {
                dps,
                duration,
                slow_factor,
                stun_chance,
                stun_duration,
                gold_chance,
                ..
            } => {
                if let Ok(mut poison) = poisons.get_mut(target) {
                    poison.stacks = poison.stacks.max(1);
                    poison.dps_per_stack = poison.dps_per_stack.max(dps);
                    poison.duration = Timer::from_seconds(duration, TimerMode::Once);
                } else {
                    commands.entity(target).insert(Poison {
                        stacks: 1,
                        dps_per_stack: dps,
                        duration: Timer::from_seconds(duration, TimerMode::Once),
                    });
                }
                commands.entity(target).insert(Slowed {
                    factor: slow_factor,
                    timer: Timer::from_seconds(duration, TimerMode::Once),
                });

                if stun_duration > 0.0 && roll_chance(&mut game.rng, stun_chance) {
                    if let Ok(mut stun) = stuns.get_mut(target) {
                        stun.timer = Timer::from_seconds(stun_duration, TimerMode::Once);
                    } else {
                        commands.entity(target).insert(Stunned {
                            timer: Timer::from_seconds(stun_duration, TimerMode::Once),
                        });
                    }
                }
                if roll_chance(&mut game.rng, gold_chance) {
                    game.coins += lucky_jade_gold_reward(game.round);
                }
            }
            _ => {}
        }

        for (start, end, color, thickness) in beams {
            spawn_beam(&mut commands, start, end, color, thickness);
        }
    }
}

fn roll_chance(rng: &mut crate::rng::OfferRng, chance: f32) -> bool {
    let threshold = (chance.clamp(0.0, 1.0) * 1000.0).round() as usize;
    rng.next_index(1000) < threshold
}

fn air_armor_multiplier(position: Vec2, flying: bool, auras: &[(Vec2, f32, f32)]) -> f32 {
    if !flying {
        return 1.0;
    }

    let armor_reduction = auras
        .iter()
        .filter(|(aura_position, range, _)| aura_position.distance(position) <= *range)
        .map(|(_, _, reduction)| *reduction)
        .fold(0.0, f32::max);
    armor_reduction_damage_multiplier(armor_reduction)
}

fn ground_armor_multiplier(position: Vec2, flying: bool, auras: &[(Vec2, f32, f32)]) -> f32 {
    if flying {
        return 1.0;
    }

    let armor_reduction = auras
        .iter()
        .filter(|(aura_position, range, _)| aura_position.distance(position) <= *range)
        .map(|(_, _, reduction)| *reduction)
        .fold(0.0, f32::max);
    armor_reduction_damage_multiplier(armor_reduction)
}

fn lucky_jade_gold_reward(round: u32) -> u32 {
    (round / 2).max(1)
}

/// Expires Gold armor reduction so the vulnerability window can be refreshed.
pub fn update_armor_break(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut armor_breaks: Query<(Entity, &mut ArmorBroken)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta().mul_f32(game.speed_multiplier());
    for (entity, mut armor_break) in &mut armor_breaks {
        armor_break.duration.tick(delta);
        if armor_break.duration.is_finished() {
            commands.entity(entity).remove::<ArmorBroken>();
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

/// Ticks burning damage and removes it when it expires.
pub fn apply_burn(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut enemies: Query<(Entity, &mut Enemy, &mut Burning)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs() * game.speed_multiplier();
    let duration_delta = time.delta().mul_f32(game.speed_multiplier());
    for (entity, mut enemy, mut burning) in &mut enemies {
        burning.duration.tick(duration_delta);
        enemy.health -= burning.dps * delta;
        if burning.duration.is_finished() {
            commands.entity(entity).remove::<Burning>();
        }
    }
}

/// Applies continuous damage from special towers such as Star Ruby.
pub fn apply_damage_auras(
    time: Res<Time>,
    game: Res<Game>,
    towers: Query<(&Transform, &Tower)>,
    mut enemies: Query<(&Transform, &mut Enemy)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs() * game.speed_multiplier();
    for (tower_transform, tower) in &towers {
        let GemEffect::DamageAura { dps } = tower.effect() else {
            continue;
        };
        let tower_position = tower_transform.translation.truncate();
        for (enemy_transform, mut enemy) in &mut enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            if enemy_transform
                .translation
                .truncate()
                .distance(tower_position)
                <= tower.range
            {
                enemy.health -= dps * delta;
            }
        }
    }
}

/// Refreshes persistent slow auras from special towers such as Uranium.
pub fn apply_slow_auras(
    mut commands: Commands,
    game: Res<Game>,
    towers: Query<(&Transform, &Tower)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut slows: Query<&mut Slowed>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (tower_transform, tower) in &towers {
        let GemEffect::SlowAura {
            factor,
            radius,
            duration,
            ..
        } = tower.effect()
        else {
            continue;
        };
        let tower_position = tower_transform.translation.truncate();
        for (enemy_entity, enemy_transform, enemy) in &enemies {
            if enemy.health <= 0.0 {
                continue;
            }
            if enemy_transform
                .translation
                .truncate()
                .distance(tower_position)
                <= radius
            {
                if let Ok(mut slow) = slows.get_mut(enemy_entity) {
                    slow.factor = slow.factor.min(factor);
                    slow.timer = Timer::from_seconds(duration, TimerMode::Once);
                } else {
                    commands.entity(enemy_entity).insert(Slowed {
                        factor,
                        timer: Timer::from_seconds(duration, TimerMode::Once),
                    });
                }
            }
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

/// Expires stun so enemies resume movement after Lucky Asian Jade procs.
pub fn update_stun(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut stuns: Query<(Entity, &mut Stunned)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (entity, mut stun) in &mut stuns {
        stun.timer
            .tick(time.delta().mul_f32(game.speed_multiplier()));
        if stun.timer.is_finished() {
            commands.entity(entity).remove::<Stunned>();
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

#[allow(clippy::type_complexity)]
pub fn update_enemy_visuals(
    game: Res<Game>,
    mut enemies: Query<(
        &Enemy,
        &mut Sprite,
        Option<&Slowed>,
        Option<&Poison>,
        Option<&Burning>,
        Option<&ArmorBroken>,
        Option<&Stunned>,
    )>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (enemy, mut sprite, slowed, poisoned, burning, armor_broken, stunned) in &mut enemies {
        let health_percent = (enemy.health / enemy.max_health).clamp(0.0, 1.0);
        sprite.color = if stunned.is_some() {
            Color::srgb(0.94, 1.0, 0.76)
        } else if slowed.is_some() {
            Color::srgb(0.40, 0.62, 0.95)
        } else if burning.is_some() {
            Color::srgb(1.0, 0.34, 0.08)
        } else if armor_broken.is_some() {
            Color::srgb(1.0, 0.82, 0.24)
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
    mut effects: Query<(
        Entity,
        &mut ShotEffect,
        &mut Transform,
        &mut Sprite,
        Option<&VfxFade>,
    )>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    let delta = time.delta_secs() * game.speed_multiplier();
    for (entity, mut effect, mut transform, mut sprite, fade) in &mut effects {
        if let Some(fade) = fade {
            let progress = (effect.timer.elapsed_secs() / fade.duration).clamp(0.0, 1.0);
            transform.translation += (fade.velocity * delta).extend(0.0);
            sprite.custom_size =
                Some(fade.start_size + (fade.end_size - fade.start_size) * progress);
            let alpha = fade.start_alpha + (fade.end_alpha - fade.start_alpha) * progress;
            sprite.color = Color::srgba(fade.rgb[0], fade.rgb[1], fade.rgb[2], alpha);
        }

        effect
            .timer
            .tick(time.delta().mul_f32(game.speed_multiplier()));
        if effect.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_beam(commands: &mut Commands, start: Vec2, end: Vec2, rgb: [f32; 3], thickness: f32) {
    let delta = end - start;
    let length = delta.length();
    if length <= 1.0 {
        return;
    }

    let midpoint = start + delta * 0.5;
    let angle = delta.y.atan2(delta.x);

    spawn_fading_sprite(
        commands,
        midpoint,
        angle,
        rgb,
        Vec2::new(length, thickness * 3.4),
        Vec2::new(length, thickness * 4.4),
        0.24,
        0.0,
        0.14,
        Vec2::ZERO,
        19.0,
    );
    spawn_fading_sprite(
        commands,
        midpoint,
        angle,
        rgb,
        Vec2::new(length, thickness),
        Vec2::new(length, thickness * 1.25),
        0.95,
        0.0,
        0.10,
        Vec2::ZERO,
        20.0,
    );
    spawn_pulse(commands, start, rgb, 14.0, 34.0, 0.18, 18.0);
    spawn_pulse(commands, end, rgb, 20.0, 58.0, 0.20, 21.0);
    spawn_sparks(commands, end, delta.normalize(), rgb);
}

#[allow(clippy::too_many_arguments)]
fn spawn_fading_sprite(
    commands: &mut Commands,
    position: Vec2,
    angle: f32,
    rgb: [f32; 3],
    start_size: Vec2,
    end_size: Vec2,
    start_alpha: f32,
    end_alpha: f32,
    duration: f32,
    velocity: Vec2,
    z: f32,
) {
    commands.spawn((
        Sprite::from_color(
            Color::srgba(rgb[0], rgb[1], rgb[2], start_alpha),
            start_size,
        ),
        Transform::from_translation(position.extend(z)).with_rotation(Quat::from_rotation_z(angle)),
        ShotEffect {
            timer: Timer::from_seconds(duration, TimerMode::Once),
        },
        VfxFade {
            duration,
            velocity,
            start_size,
            end_size,
            rgb,
            start_alpha,
            end_alpha,
        },
        GameWorld,
    ));
}

fn spawn_pulse(
    commands: &mut Commands,
    position: Vec2,
    rgb: [f32; 3],
    start_size: f32,
    end_size: f32,
    duration: f32,
    z: f32,
) {
    spawn_fading_sprite(
        commands,
        position,
        std::f32::consts::FRAC_PI_4,
        rgb,
        Vec2::splat(start_size),
        Vec2::splat(end_size),
        0.42,
        0.0,
        duration,
        Vec2::ZERO,
        z,
    );
}

fn spawn_sparks(commands: &mut Commands, position: Vec2, direction: Vec2, rgb: [f32; 3]) {
    let base_angle = direction.y.atan2(direction.x);
    for index in 0..6 {
        let offset = -0.9 + index as f32 * 0.36;
        let angle = base_angle + offset;
        let velocity = Vec2::new(angle.cos(), angle.sin()) * (90.0 + index as f32 * 13.0);
        spawn_fading_sprite(
            commands,
            position,
            angle,
            rgb,
            Vec2::new(14.0, 3.0),
            Vec2::new(4.0, 1.0),
            0.72,
            0.0,
            0.22,
            velocity,
            22.0,
        );
    }
}
