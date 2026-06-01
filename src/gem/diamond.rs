use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Strong single-target damage that hits hardest against ground enemies.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Diamond",
        srgb: [0.86, 0.96, 1.0],
        chipped: TowerStats::new(15.0, 118.0, 0.85),
        effect,
    }
}

/// Bonus damage to grounded enemies grows with grade.
fn effect(grade: GemGrade) -> GemEffect {
    GemEffect::Favored {
        air: false,
        multiplier: 1.5 + 0.15 * grade.tier() as f32,
    }
}
