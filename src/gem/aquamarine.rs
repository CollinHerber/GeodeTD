use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Smallest range in the game paired with a split-second cooldown, giving it the
/// highest single-target damage per second. No special on-hit behavior.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Aquamarine",
        srgb: [0.32, 0.83, 0.78],
        chipped: TowerStats::new(9.0, 78.0, 0.22),
        effect,
    }
}

fn effect(_grade: GemGrade) -> GemEffect {
    GemEffect::None
}
