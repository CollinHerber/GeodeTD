use bevy::prelude::*;
use std::time::Duration;

use crate::game::RoundKind;
use crate::gem::{GemEffect, GemGrade, GemKind, SpecialGem, TowerStats, special_recipe_for_source};

#[derive(Component)]
pub struct PathMarker;

#[derive(Component)]
pub struct CheckpointMarker;

#[derive(Component)]
pub struct Tower {
    pub gem: GemKind,
    pub grade: GemGrade,
    pub special: Option<SpecialGem>,
    pub damage: f32,
    pub range: f32,
    pub cooldown: Timer,
    pub black_opal_boosted: bool,
    pub yellow_sapphire_boosted: bool,
}

impl Tower {
    pub fn display_name(&self) -> String {
        if let Some(special) = self.special {
            special.name().to_string()
        } else {
            format!("{} {}", self.grade.name(), self.gem.name())
        }
    }

    pub fn action_label(&self) -> String {
        if let Some(special) = self.special {
            return match special.upgrade() {
                Some(next) => match special.upgrade_cost() {
                    Some(cost) => format!("Upgrade to {} ({}g)", next.name(), cost),
                    None => format!("Upgrade to {}", next.name()),
                },
                None => "Special complete".to_string(),
            };
        }

        if let Some(recipe) = special_recipe_for_source(self.gem, self.grade) {
            format!("Combine {}", recipe.result.name())
        } else {
            match self.grade.next() {
                Some(next) => format!("Upgrade to {}", next.name()),
                None => "Perfect grade".to_string(),
            }
        }
    }

    pub fn attack_damage(&self) -> f32 {
        if self.special.is_some() {
            self.damage
        } else {
            self.damage * self.grade.damage_multiplier()
        }
    }

    pub fn effect(&self) -> GemEffect {
        self.special
            .map_or_else(|| self.gem.effect(self.grade), SpecialGem::effect)
    }

    pub fn srgb(&self) -> [f32; 3] {
        self.special
            .map_or_else(|| self.gem.srgb(), SpecialGem::srgb)
    }

    pub fn is_basic(&self, gem: GemKind, grade: GemGrade) -> bool {
        self.special.is_none() && self.gem == gem && self.grade == grade
    }

    pub fn can_regular_upgrade(&self) -> bool {
        self.special.is_none() && self.grade.next().is_some()
    }

    pub fn range_indicator_radius(&self) -> Option<f32> {
        match self.effect() {
            GemEffect::Haste { .. } => Some(self.range),
            GemEffect::DamageBoost { range, .. } => Some(range),
            GemEffect::SlowSplashBoost { boost_range, .. } => Some(boost_range),
            GemEffect::Tourmaline { armor_range, .. } => Some(armor_range),
            GemEffect::SlowAura { radius, .. } => Some(radius),
            _ => None,
        }
    }

    pub fn become_special(&mut self, special: SpecialGem) {
        self.special = Some(special);
        self.gem = match special {
            SpecialGem::BlackOpal | SpecialGem::MysticBlackOpal => GemKind::Opal,
            SpecialGem::BloodStone | SpecialGem::AncientBloodStone => GemKind::Ruby,
            SpecialGem::Gold | SpecialGem::EgyptianGold => GemKind::Amethyst,
            SpecialGem::Jade | SpecialGem::AsianJade | SpecialGem::LuckyAsianJade => {
                GemKind::Emerald
            }
            SpecialGem::Malachite | SpecialGem::VividMalachite | SpecialGem::MightyMalachite => {
                GemKind::Opal
            }
            SpecialGem::PinkDiamond | SpecialGem::GreatPinkDiamond => GemKind::Diamond,
            SpecialGem::RedCrystal
            | SpecialGem::RedCrystalFacet
            | SpecialGem::RoseQuartzCrystal => GemKind::Emerald,
            SpecialGem::Silver | SpecialGem::SterlingSilver | SpecialGem::SilverKnight => {
                GemKind::Sapphire
            }
            SpecialGem::StarRuby | SpecialGem::BloodStar | SpecialGem::FireStar => {
                GemKind::Amethyst
            }
            SpecialGem::Tourmaline | SpecialGem::ParaibaTourmaline => GemKind::Aquamarine,
            SpecialGem::Uranium238 | SpecialGem::Uranium235 => GemKind::Topaz,
            SpecialGem::YellowSapphire | SpecialGem::StarYellowSapphire => GemKind::Sapphire,
        };
        self.grade = special.sprite_grade();
        self.black_opal_boosted = false;
        self.yellow_sapphire_boosted = false;
        self.apply_stats(special.stats());
    }

    fn apply_stats(&mut self, stats: TowerStats) {
        self.damage = stats.damage;
        self.range = stats.range;
        self.cooldown = Timer::from_seconds(stats.cooldown, TimerMode::Once);
        self.cooldown
            .set_elapsed(Duration::from_secs_f32(stats.cooldown));
    }
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

#[derive(Component)]
pub struct Stunned {
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
pub struct Burning {
    pub dps: f32,
    pub duration: Timer,
}

#[derive(Component)]
pub struct ArmorBroken {
    pub reduction: f32,
    pub duration: Timer,
}

impl ArmorBroken {
    pub fn damage_multiplier(&self) -> f32 {
        armor_reduction_damage_multiplier(self.reduction)
    }
}

pub fn armor_reduction_damage_multiplier(reduction: f32) -> f32 {
    // The original game has a Warcraft-style armor table. This prototype
    // approximates the wiki note that -14 armor roughly doubles damage.
    1.0 + reduction / 14.0
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

/// Top-bar button that opens/closes the upgrading-chances panel.
#[derive(Component)]
pub struct ChancesButton;

/// Root of the upgrading-chances panel (toggled open/closed).
#[derive(Component)]
pub struct ChancesPanel;

/// A "+10%" buy button for one chance tier (0 → Flawed … 3 → Perfect).
#[derive(Component)]
pub struct ChanceBuyButton {
    pub tier: usize,
}

/// The label for one tier row, refreshed with its current odds.
#[derive(Component)]
pub struct ChanceRowText {
    pub tier: usize,
}

/// The label inside a tier's buy button, refreshed with the next cost.
#[derive(Component)]
pub struct ChanceBuyText {
    pub tier: usize,
}

/// Header line of the chances panel showing current gold / Chipped odds.
#[derive(Component)]
pub struct ChancesHeaderText;

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
pub struct PlayTypeScreen;

#[derive(Component, Clone, Copy)]
pub struct MultiplayerMenuScreen;

#[derive(Component, Clone, Copy)]
pub struct DisplayNameScreen;

#[derive(Component, Clone, Copy)]
pub struct HostLobbyScreen;

#[derive(Component, Clone, Copy)]
pub struct JoinLobbyScreen;

#[derive(Component, Clone, Copy)]
pub struct WaitingLobbyScreen;

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

#[derive(Component)]
pub struct LobbyRowButton {
    pub lobby_id: String,
    pub center: Vec2,
    pub size: Vec2,
}

#[derive(Component, Clone, Copy)]
pub struct TextInputDisplay {
    pub field: TextInputField,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TextInputField {
    DisplayName,
    LobbyName,
}

#[derive(Component, Clone, Copy)]
pub struct MultiplayerStatusText;

#[derive(Component, Clone, Copy)]
pub struct MultiplayerDynamic;

#[derive(Clone, Copy)]
pub enum MenuAction {
    Play,
    SinglePlayer,
    Multiplayer,
    Host,
    Join,
    SubmitDisplayName,
    CreateLobby,
    StartMultiplayerGame,
    LeaveLobby,
    Standard,
    Random,
    HowToPlay,
    Settings,
    TogglePathOverlay,
    Quit,
    Home,
}
