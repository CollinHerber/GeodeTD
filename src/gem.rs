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
            GemKind::Ruby => TowerStats::new(16.0, 132.0, 0.86),
            GemKind::Sapphire => TowerStats::new(10.0, 150.0, 0.72),
            GemKind::Topaz => TowerStats::new(7.0, 118.0, 0.36),
            GemKind::Emerald => TowerStats::new(12.0, 178.0, 0.82),
            GemKind::Amethyst => TowerStats::new(22.0, 112.0, 1.15),
            GemKind::Diamond => TowerStats::new(13.0, 146.0, 0.68),
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
