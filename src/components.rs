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

#[derive(Component)]
pub struct EnemyAnimation {
    pub first: usize,
    pub last: usize,
    pub timer: Timer,
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
pub struct VfxFade {
    pub duration: f32,
    pub velocity: Vec2,
    pub start_size: Vec2,
    pub end_size: Vec2,
    pub rgb: [f32; 3],
    pub start_alpha: f32,
    pub end_alpha: f32,
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
pub struct OfferSelectionGlow {
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

#[derive(Component)]
pub struct ShowRangeButton;

#[derive(Component)]
pub struct ShowRangeButtonText;

/// The button that commits the player's chosen starter once all five are placed.
#[derive(Component)]
pub struct ConfirmKeepButton;

/// The label inside the upgrade button, so its color can be dimmed when the
/// upgrade is unavailable.
#[derive(Component)]
pub struct UpgradeButtonText;

/// A transient halo drawn behind a tower that can be sacrificed for the current
/// upgrade. Rebuilt every frame while an upgrade is in progress.
#[derive(Component)]
pub struct UpgradeHighlight;

#[derive(Component)]
pub struct AuraRangeSegment;

#[derive(Component)]
pub struct PlacementPreview;

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
    TogglePathOverlay,
    Home,
}
