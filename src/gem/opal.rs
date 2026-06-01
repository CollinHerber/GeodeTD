use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Below-average single-target damage, but a support aura that speeds up the
/// attacks of every tower within its range.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Opal",
        srgb: [0.62, 0.84, 0.95],
        chipped: TowerStats::new(6.0, 150.0, 1.0),
        effect,
    }
}

/// Higher grades grant a larger cooldown reduction to nearby towers.
fn effect(grade: GemGrade) -> GemEffect {
    GemEffect::Haste {
        cooldown_reduction: 0.15 + 0.05 * grade.tier() as f32,
    }
}
