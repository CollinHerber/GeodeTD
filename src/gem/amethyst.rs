use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Top-tier single-target damage and large range; devastating against air.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Amethyst",
        srgb: [0.68, 0.22, 0.92],
        chipped: TowerStats::new(20.0, 200.0, 1.0),
        effect,
    }
}

/// Bonus damage to airborne enemies grows with grade.
fn effect(grade: GemGrade) -> GemEffect {
    GemEffect::Favored {
        air: true,
        multiplier: 2.0 + 0.25 * grade.tier() as f32,
    }
}
