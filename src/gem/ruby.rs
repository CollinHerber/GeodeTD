use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Large damage, slow attack, medium range — plus splash on hit.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Ruby",
        srgb: [0.92, 0.08, 0.12],
        chipped: TowerStats::new(24.0, 130.0, 1.0),
        effect,
    }
}

/// Splash is a flat fraction of the hit, independent of grade.
fn effect(_grade: GemGrade) -> GemEffect {
    GemEffect::Splash {
        radius: 50.0,
        damage_fraction: 0.5,
    }
}
