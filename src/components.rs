use bevy::prelude::*;

use crate::gem::{GemGrade, GemKind};

#[derive(Component)]
pub struct PathMarker;

#[derive(Component)]
pub struct CheckpointMarker;

#[derive(Component)]
pub struct Tower {
    pub gem: GemKind,
    pub grade: GemGrade,
    pub damage: f32,
    pub range: f32,
    pub cooldown: Timer,
}

#[derive(Component)]
pub struct Enemy {
    pub next_path_index: usize,
    pub health: f32,
    pub max_health: f32,
    pub speed: f32,
}

#[derive(Component)]
pub struct ShotEffect {
    pub timer: Timer,
}

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct EscapeMenu;

#[derive(Component)]
pub struct OfferVisual {
    pub index: usize,
}

#[derive(Component)]
pub struct OfferLabel {
    pub index: usize,
}

#[derive(Component)]
pub struct SelectionMenu;

#[derive(Component)]
pub struct UpgradeButton;

#[derive(Component, Clone, Copy)]
pub struct HomeScreen;

#[derive(Component, Clone, Copy)]
pub struct SettingsScreen;

#[derive(Component, Clone, Copy)]
pub struct ModeSelectScreen;

#[derive(Component, Clone, Copy)]
pub struct HowToPlayScreen;

#[derive(Component, Clone, Copy)]
pub struct GameWorld;

#[derive(Component)]
pub struct MenuButton {
    pub action: MenuAction,
    pub center: Vec2,
    pub size: Vec2,
}

#[derive(Clone, Copy)]
pub enum MenuAction {
    Play,
    Standard,
    Random,
    HowToPlay,
    Settings,
    Home,
}
