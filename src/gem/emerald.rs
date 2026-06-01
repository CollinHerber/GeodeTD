use super::{GemDef, GemEffect, GemGrade, TowerStats};

/// Low hit damage, below-average range — applies a stacking, slowing poison.
pub(super) fn def() -> GemDef {
    GemDef {
        name: "Emerald",
        srgb: [0.08, 0.72, 0.34],
        chipped: TowerStats::new(6.0, 128.0, 1.0),
        effect,
    }
}

/// Higher grades poison harder and slow a touch more.
fn effect(grade: GemGrade) -> GemEffect {
    let tier = grade.tier() as f32;
    GemEffect::Poison {
        dps_per_stack: 4.0 + tier,
        duration: 3.0,
        max_stacks: 5,
        slow_factor: (0.85 - 0.02 * tier).max(0.7),
    }
}
