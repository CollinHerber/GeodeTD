use bevy::prelude::*;

use crate::game::RoundKind;
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
pub struct StarterCandidate;

#[derive(Component)]
pub struct StarterWall;

#[derive(Component)]
pub struct Enemy {
    pub next_path_index: usize,
    pub health: f32,
    pub max_health: f32,
    pub speed: f32,
    pub kind: RoundKind,
    /// Flying enemies ignore the maze path and head straight for the finish.
    pub flying: bool,
}

/// Temporary movement slow applied by Sapphire towers. Reapplying refreshes it.
#[derive(Component)]
pub struct Slowed {
    pub factor: f32,
    pub timer: Timer,
}

/// Stacking damage-over-time applied by Emerald towers.
#[derive(Component)]
pub struct Poison {
    pub stacks: u32,
    pub dps_per_stack: f32,
    pub duration: Timer,
}

#[derive(Component)]
pub struct ShotEffect {
    pub timer: Timer,
}

#[derive(Component)]
pub struct HudText;

#[derive(Component)]
pub struct TopBarText;

/// Title line of the left-side round information panel.
#[derive(Component)]
pub struct RoundInfoTitle;

/// Body (stats) of the left-side round information panel.
#[derive(Component)]
pub struct RoundInfoBody;

#[derive(Component)]
pub struct SpeedButton;

#[derive(Component)]
pub struct SpeedText;

#[derive(Component)]
pub struct EscapeMenu;

#[derive(Component)]
pub struct EscapeMenuInfo;

#[derive(Component)]
pub struct EscapeMenuButton {
    pub action: EscapeMenuAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EscapeMenuAction {
    Reset,
    Home,
    HowToPlay,
}

#[derive(Component)]
pub struct OfferVisual {
    pub index: usize,
}

#[derive(Component)]
pub struct OfferLabel {
    pub index: usize,
}

#[derive(Component)]
pub struct OfferButton {
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
