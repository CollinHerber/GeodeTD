use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Low damage, above-average range — applies a ~20% slowing debuff on hit.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Sapphire",
        srgb: [0.12, 0.33, 0.95],
        chipped: TowerStats::new(7.0, 180.0, 0.8),
        effect,
    }
}

/// Starts at 20% slow and deepens with grade.
fn effect(grade: GemGrade) -> GemEffect {
    let tier = grade.tier() as f32;
    GemEffect::Slow {
        factor: (0.8 - 0.03 * tier).max(0.6),
        duration: 1.5,
    }
}
