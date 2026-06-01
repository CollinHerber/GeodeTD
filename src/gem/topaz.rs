use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Fast, low-damage poke that strikes several separate enemies at once.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Topaz",
        srgb: [1.0, 0.74, 0.12],
        chipped: TowerStats::new(7.0, 118.0, 0.5),
        effect,
    }
}

/// Higher grades strike more targets; the extra targets take reduced damage.
fn effect(grade: GemGrade) -> GemEffect {
    GemEffect::Multi {
        targets: 3 + grade.tier() as u32,
        damage_fraction: 0.8,
    }
}
