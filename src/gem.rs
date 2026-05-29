use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GemKind {
    Ruby,
    Sapphire,
    Topaz,
    Emerald,
    Amethyst,
    Diamond,
}

impl GemKind {
    pub fn name(self) -> &'static str {
        match self {
            GemKind::Ruby => "Ruby",
            GemKind::Sapphire => "Sapphire",
            GemKind::Topaz => "Topaz",
            GemKind::Emerald => "Emerald",
            GemKind::Amethyst => "Amethyst",
            GemKind::Diamond => "Diamond",
        }
    }

    pub fn color(self) -> Color {
        match self {
            GemKind::Ruby => Color::srgb(0.92, 0.08, 0.12),
            GemKind::Sapphire => Color::srgb(0.12, 0.33, 0.95),
            GemKind::Topaz => Color::srgb(1.0, 0.74, 0.12),
            GemKind::Emerald => Color::srgb(0.08, 0.72, 0.34),
            GemKind::Amethyst => Color::srgb(0.68, 0.22, 0.92),
            GemKind::Diamond => Color::srgb(0.86, 0.96, 1.0),
        }
    }

    pub fn chipped_stats(self) -> TowerStats {
        match self {
            // Large damage, slow attack, medium range — plus a small splash.
            GemKind::Ruby => TowerStats::new(26.0, 134.0, 1.15),
            // Low damage, high range, average attack speed — plus a slow.
            GemKind::Sapphire => TowerStats::new(7.0, 172.0, 0.8),
            // Fast, low-damage poke (unchanged baseline).
            GemKind::Topaz => TowerStats::new(7.0, 118.0, 0.36),
            // Low hit damage, slow attack, medium range — plus stacking poison.
            GemKind::Emerald => TowerStats::new(6.0, 138.0, 1.0),
            // High range, low damage, medium attack speed.
            GemKind::Amethyst => TowerStats::new(8.0, 190.0, 0.8),
            // Average damage and attack speed, small range — plus crits.
            GemKind::Diamond => TowerStats::new(13.0, 112.0, 0.8),
        }
    }

    /// The special on-hit behavior for this gem, scaled by the tower's grade.
    pub fn effect(self, grade: GemGrade) -> GemEffect {
        let tier = grade.tier() as f32;
        match self {
            GemKind::Diamond => GemEffect::Crit {
                chance: 0.05 + 0.03 * tier,
                multiplier: 2.0,
            },
            GemKind::Sapphire => GemEffect::Slow {
                factor: (0.6 - 0.05 * tier).max(0.3),
                duration: 1.5,
            },
            GemKind::Ruby => GemEffect::Splash {
                radius: 46.0,
                damage_fraction: 0.5,
            },
            GemKind::Emerald => GemEffect::Poison {
                dps_per_stack: 4.0 + tier,
                duration: 3.0,
                max_stacks: 5,
            },
            GemKind::Topaz => GemEffect::Chain {
                chance: 0.05 + 0.02 * tier,
                jumps: 3 + grade.tier() as u32,
                damage_fraction: 0.6,
            },
            GemKind::Amethyst => GemEffect::None,
        }
    }
}

/// On-hit behavior layered on top of a tower's base damage. Magnitudes already
/// account for grade where relevant (see `GemKind::effect`).
#[derive(Clone, Copy)]
pub enum GemEffect {
    None,
    /// Chance in `[0, 1]` to multiply a hit's damage by `multiplier`.
    Crit { chance: f32, multiplier: f32 },
    /// Multiplies enemy speed by `factor` (< 1.0) for `duration` seconds.
    Slow { factor: f32, duration: f32 },
    /// Deals `damage_fraction` of the hit to other enemies within `radius`.
    Splash { radius: f32, damage_fraction: f32 },
    /// Stacking damage-over-time; each hit adds a stack up to `max_stacks` and
    /// refreshes the `duration`.
    Poison {
        dps_per_stack: f32,
        duration: f32,
        max_stacks: u32,
    },
    /// Chance to arc to up to `jumps` nearby enemies, each taking
    /// `damage_fraction` of the hit.
    Chain {
        chance: f32,
        jumps: u32,
        damage_fraction: f32,
    },
}

impl GemEffect {
    /// Short one-line summary for the tower stat panel.
    pub fn describe(self) -> String {
        match self {
            GemEffect::None => "Effect: none".to_string(),
            GemEffect::Crit { chance, multiplier } => {
                format!("Crit: {:.0}% for x{:.0}", chance * 100.0, multiplier)
            }
            GemEffect::Slow { factor, duration } => {
                format!("Slow: -{:.0}% for {:.1}s", (1.0 - factor) * 100.0, duration)
            }
            GemEffect::Splash {
                radius,
                damage_fraction,
            } => format!("Splash: {:.0}% in r{:.0}", damage_fraction * 100.0, radius),
            GemEffect::Poison {
                dps_per_stack,
                max_stacks,
                ..
            } => format!("Poison: {:.0}/s up to {}x", dps_per_stack, max_stacks),
            GemEffect::Chain { chance, jumps, .. } => {
                format!("Chain: {:.0}% to {} targets", chance * 100.0, jumps)
            }
        }
    }
}

pub const GEM_KINDS: [GemKind; 6] = [
    GemKind::Ruby,
    GemKind::Sapphire,
    GemKind::Topaz,
    GemKind::Emerald,
    GemKind::Amethyst,
    GemKind::Diamond,
];

pub const GRADE_LADDER: [GemGrade; 5] = [
    GemGrade::Chipped,
    GemGrade::Flawed,
    GemGrade::Regular,
    GemGrade::Cut,
    GemGrade::Perfect,
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GemGrade {
    Chipped,
    Flawed,
    Regular,
    Cut,
    Perfect,
}

impl GemGrade {
    pub fn name(self) -> &'static str {
        match self {
            GemGrade::Chipped => "Chipped",
            GemGrade::Flawed => "Flawed",
            GemGrade::Regular => "Regular",
            GemGrade::Cut => "Cut",
            GemGrade::Perfect => "Perfect",
        }
    }

    /// Position on the `GRADE_LADDER`, `0` (Chipped) through `4` (Perfect).
    pub fn tier(self) -> usize {
        match self {
            GemGrade::Chipped => 0,
            GemGrade::Flawed => 1,
            GemGrade::Regular => 2,
            GemGrade::Cut => 3,
            GemGrade::Perfect => 4,
        }
    }

    pub fn next(self) -> Option<GemGrade> {
        match self {
            GemGrade::Chipped => Some(GemGrade::Flawed),
            GemGrade::Flawed => Some(GemGrade::Regular),
            GemGrade::Regular => Some(GemGrade::Cut),
            GemGrade::Cut => Some(GemGrade::Perfect),
            GemGrade::Perfect => None,
        }
    }

    pub fn damage_multiplier(self) -> f32 {
        match self {
            GemGrade::Chipped => 1.0,
            GemGrade::Flawed => 1.45,
            GemGrade::Regular => 2.05,
            GemGrade::Cut => 2.9,
            GemGrade::Perfect => 4.1,
        }
    }

    pub fn size_multiplier(self) -> f32 {
        match self {
            GemGrade::Chipped => 1.0,
            GemGrade::Flawed => 1.08,
            GemGrade::Regular => 1.16,
            GemGrade::Cut => 1.24,
            GemGrade::Perfect => 1.34,
        }
    }
}

#[derive(Clone, Copy)]
pub struct TowerStats {
    pub damage: f32,
    pub range: f32,
    pub cooldown: f32,
}

impl TowerStats {
    pub fn new(damage: f32, range: f32, cooldown: f32) -> Self {
        Self {
            damage,
            range,
            cooldown,
        }
    }
}
