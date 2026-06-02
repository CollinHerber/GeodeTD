use bevy::prelude::*;

// Each gem's data and on-hit behavior lives in its own file under `gem/` so the
// kinds stay separated by concern. `GemKind` below is the shared handle the rest
// of the game uses; it just dispatches to the per-gem `def()`.
mod amethyst;
mod aquamarine;
mod diamond;
mod emerald;
mod opal;
mod ruby;
mod sapphire;
mod topaz;

const TOWER_RANGE_MULTIPLIER: f32 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GemKind {
    Ruby,
    Sapphire,
    Topaz,
    Emerald,
    Amethyst,
    Diamond,
    Aquamarine,
    Opal,
}

/// The full static definition of a gem, assembled in its own module under `gem/`.
/// Private to this module — the outside world goes through `GemKind`'s methods.
struct GemDef {
    name: &'static str,
    srgb: [f32; 3],
    /// Base (Chipped) tower stats before any grade multiplier.
    chipped: TowerStats,
    /// The special on-hit behavior, scaled by the tower's grade.
    effect: fn(GemGrade) -> GemEffect,
}

impl GemKind {
    /// Single dispatch point: adding a gem means one arm here plus its file.
    fn def(self) -> GemDef {
        match self {
            GemKind::Ruby => ruby::def(),
            GemKind::Sapphire => sapphire::def(),
            GemKind::Topaz => topaz::def(),
            GemKind::Emerald => emerald::def(),
            GemKind::Amethyst => amethyst::def(),
            GemKind::Diamond => diamond::def(),
            GemKind::Aquamarine => aquamarine::def(),
            GemKind::Opal => opal::def(),
        }
    }

    pub fn name(self) -> &'static str {
        self.def().name
    }

    pub fn srgb(self) -> [f32; 3] {
        self.def().srgb
    }

    pub fn chipped_stats(self) -> TowerStats {
        self.def().chipped
    }

    /// The special on-hit behavior for this gem, scaled by the tower's grade.
    pub fn effect(self, grade: GemGrade) -> GemEffect {
        (self.def().effect)(grade)
    }
}

/// Special behavior layered on top of a tower's base damage. Magnitudes already
/// account for grade where relevant (see each gem module's `effect`).
#[derive(Clone, Copy)]
pub enum GemEffect {
    None,
    /// Multiplies enemy speed by `factor` (< 1.0) for `duration` seconds.
    Slow {
        factor: f32,
        duration: f32,
    },
    /// Deals `damage_fraction` of the hit to other enemies within `radius`.
    Splash {
        radius: f32,
        damage_fraction: f32,
    },
    /// Stacking damage-over-time; each hit adds a stack up to `max_stacks` and
    /// refreshes the `duration`. Also slows by `slow_factor` (a slowing poison).
    Poison {
        dps_per_stack: f32,
        duration: f32,
        max_stacks: u32,
        slow_factor: f32,
    },
    /// Strikes up to `targets` separate enemies in range, the extras taking
    /// `damage_fraction` of the hit.
    Multi {
        targets: u32,
        damage_fraction: f32,
    },
    /// Deals `multiplier`x damage to its favored enemy class — air when `air` is
    /// true, ground otherwise — and normal damage to the rest.
    Favored {
        air: bool,
        multiplier: f32,
    },
    /// Support aura: reduces the cooldown of towers within range by
    /// `cooldown_reduction` (a fraction in `[0, 1]`). Does nothing on hit.
    Haste {
        cooldown_reduction: f32,
    },
}

impl GemEffect {
    /// Short one-line summary for the tower stat panel.
    pub fn describe(self) -> String {
        match self {
            GemEffect::None => "Effect: none".to_string(),
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
                slow_factor,
                ..
            } => format!(
                "Poison: {:.0}/s up to {}x, -{:.0}% slow",
                dps_per_stack,
                max_stacks,
                (1.0 - slow_factor) * 100.0
            ),
            GemEffect::Multi { targets, .. } => format!("Hits up to {} targets", targets),
            GemEffect::Favored { air, multiplier } => format!(
                "vs {}: x{:.1} damage",
                if air { "air" } else { "ground" },
                multiplier
            ),
            GemEffect::Haste { cooldown_reduction } => {
                format!("Cooldowns -{:.0}% in range", cooldown_reduction * 100.0)
            }
        }
    }
}

pub const GEM_KINDS: [GemKind; 8] = [
    GemKind::Ruby,
    GemKind::Sapphire,
    GemKind::Topaz,
    GemKind::Emerald,
    GemKind::Amethyst,
    GemKind::Diamond,
    GemKind::Aquamarine,
    GemKind::Opal,
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
            range: range * TOWER_RANGE_MULTIPLIER,
            cooldown,
        }
    }
}
