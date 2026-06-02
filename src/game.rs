use bevy::prelude::*;
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::constants::{OFFER_COUNT, WAVE_COUNTDOWN_SECONDS};
use crate::gem::{GEM_KINDS, GemKind};
use crate::rng::OfferRng;

#[derive(Resource)]
pub struct Game {
    pub screen: AppScreen,
    pub mode: GameMode,
    pub phase: Phase,
    pub round: u32,
    pub lives: i32,
    pub coins: u32,
    pub offers: [Option<GemKind>; OFFER_COUNT],
    pub selected_offer: Option<usize>,
    pub placed_starters: [Option<Entity>; OFFER_COUNT],
    pub pending_enemies: u32,
    pub spawn_timer: Timer,
    pub countdown_timer: Timer,
    pub rng: OfferRng,
    pub message: String,
    pub selected_tower: Option<Entity>,
    pub upgrade_source: Option<Entity>,
    /// A placed starter the player has selected but not yet confirmed to keep.
    pub keep_candidate: Option<Entity>,
    pub shown_aura_ranges: HashSet<Entity>,
    pub show_path_overlay: bool,
    pub paused: bool,
    pub speed: u8,
}

impl Game {
    pub fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|time| time.as_nanos() as u64)
            .unwrap_or(0x5EED);
        let mut rng = OfferRng::new(seed);

        Self {
            screen: AppScreen::Home,
            mode: GameMode::Standard,
            phase: Phase::Build,
            round: 1,
            lives: 20,
            coins: 0,
            offers: random_offers(&mut rng),
            selected_offer: None,
            placed_starters: [None; OFFER_COUNT],
            pending_enemies: 0,
            spawn_timer: ready_timer(0.65, TimerMode::Repeating),
            countdown_timer: Timer::from_seconds(WAVE_COUNTDOWN_SECONDS, TimerMode::Once),
            rng,
            message: "Place all five chipped gems, then choose one to keep.".to_string(),
            selected_tower: None,
            upgrade_source: None,
            keep_candidate: None,
            shown_aura_ranges: HashSet::new(),
            show_path_overlay: true,
            paused: false,
            speed: 1,
        }
    }

    pub fn selected_gem(&self) -> Option<GemKind> {
        self.selected_offer.and_then(|index| self.offers[index])
    }

    pub fn speed_multiplier(&self) -> f32 {
        self.speed as f32
    }

    pub fn cycle_speed(&mut self) {
        self.speed = match self.speed {
            1 => 2,
            2 => 3,
            _ => 1,
        };
    }

    pub fn refresh_offers(&mut self) {
        self.offers = random_offers(&mut self.rng);
        self.selected_offer = None;
        self.placed_starters = [None; OFFER_COUNT];
        self.keep_candidate = None;
    }

    pub fn clear_offers(&mut self) {
        self.offers = [None; OFFER_COUNT];
        self.selected_offer = None;
        self.placed_starters = [None; OFFER_COUNT];
        self.keep_candidate = None;
    }

    pub fn toggle_aura_range(&mut self, tower: Entity) {
        if !self.shown_aura_ranges.insert(tower) {
            self.shown_aura_ranges.remove(&tower);
        }
    }

    pub fn all_starters_placed(&self) -> bool {
        self.placed_starters.iter().all(Option::is_some)
    }

    pub fn placed_starter_count(&self) -> usize {
        self.placed_starters
            .iter()
            .filter(|starter| starter.is_some())
            .count()
    }

    pub fn begin_countdown(&mut self, gem: GemKind) {
        self.phase = Phase::Countdown;
        self.clear_offers();
        self.selected_tower = None;
        self.upgrade_source = None;
        self.countdown_timer = Timer::from_seconds(WAVE_COUNTDOWN_SECONDS, TimerMode::Once);
        self.message = format!(
            "Kept a Chipped {}. Wave starts in {:.0} seconds.",
            gem.name(),
            WAVE_COUNTDOWN_SECONDS
        );
    }

    pub fn begin_wave(&mut self) {
        self.phase = Phase::Wave;
        let plan = RoundPlan::for_round(self.round);
        self.pending_enemies = plan.count;
        self.spawn_timer = ready_timer(0.65, TimerMode::Repeating);
        self.message = match plan.kind {
            RoundKind::Normal => {
                format!("Wave {} is moving through the checkpoints.", self.round)
            }
            RoundKind::Swift => format!("Wave {} is fast but fragile — hold fast.", self.round),
            RoundKind::Flying => {
                format!("Wave {} takes to the air and ignores the maze.", self.round)
            }
            RoundKind::Boss => {
                format!("Boss wave {}! A single massive foe approaches.", self.round)
            }
        };
    }

    pub fn begin_build_round(&mut self) {
        self.phase = Phase::Build;
        self.round += 1;
        self.refresh_offers();
        self.upgrade_source = None;
        let kind = RoundPlan::for_round(self.round).kind;
        self.message = if kind == RoundKind::Normal {
            format!(
                "Round {}: place all five chipped gems, then choose one to keep.",
                self.round
            )
        } else {
            format!(
                "Round {} ({} ahead): place all five chipped gems, then choose one to keep.",
                self.round,
                kind.label()
            )
        };
    }

    pub fn reset_for_mode(&mut self, mode: GameMode) {
        self.mode = mode;
        self.phase = Phase::Build;
        self.round = 1;
        self.lives = 20;
        self.coins = 0;
        self.pending_enemies = 0;
        self.refresh_offers();
        self.selected_tower = None;
        self.upgrade_source = None;
        self.keep_candidate = None;
        self.shown_aura_ranges.clear();
        self.paused = false;
        self.speed = 1;
        self.message = format!(
            "{} mode: place all five chipped gems, then choose one to keep.",
            mode.name()
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Home,
    ModeSelect,
    HowToPlay,
    Settings,
    Playing,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Standard,
    Random,
}

impl GameMode {
    pub fn name(self) -> &'static str {
        match self {
            GameMode::Standard => "Standard",
            GameMode::Random => "Random",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Build,
    Countdown,
    Wave,
}

/// The flavor of a wave. Derived purely from the round number so the info panel
/// and the spawner always agree on what is coming.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RoundKind {
    Normal,
    Swift,
    Flying,
    Boss,
}

impl RoundKind {
    pub fn label(self) -> &'static str {
        match self {
            RoundKind::Normal => "Standard Wave",
            RoundKind::Swift => "Swift Wave",
            RoundKind::Flying => "Flying Wave",
            RoundKind::Boss => "Boss Wave",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            RoundKind::Normal => "A standard column of grounded foes.",
            RoundKind::Swift => "Fast but fragile — they sprint the maze.",
            RoundKind::Flying => "Ignores the maze and flies straight across.",
            RoundKind::Boss => "A single massive foe with enormous health.",
        }
    }

    /// Accent color used for the info-panel title and as the enemy's base tint.
    pub fn accent(self) -> Color {
        match self {
            RoundKind::Normal => Color::srgb(0.90, 0.40, 0.34),
            RoundKind::Swift => Color::srgb(0.98, 0.72, 0.20),
            RoundKind::Flying => Color::srgb(0.46, 0.82, 0.92),
            RoundKind::Boss => Color::srgb(0.80, 0.40, 0.92),
        }
    }
}

/// Resolved per-enemy stats and head count for a single round. Computed from the
/// round number in one place ([`RoundPlan::for_round`]) so spawning and the UI
/// never drift apart.
#[derive(Clone, Copy)]
pub struct RoundPlan {
    pub kind: RoundKind,
    pub count: u32,
    pub health: f32,
    pub speed: f32,
    pub flying: bool,
}

impl RoundPlan {
    pub fn for_round(round: u32) -> Self {
        let mut base_count = 5 + round * 2;
        let mut base_health = 30.0 + round as f32 * 8.0;
        let mut base_speed = 72.0 + round as f32 * 3.0;

        match round {
            1 => {
                base_count = 5;
                base_health *= 0.75;
                base_speed *= 0.92;
            }
            2 => {
                base_count = 7;
                base_health *= 0.85;
                base_speed *= 0.95;
            }
            _ => {}
        }

        // Boss every 20 rounds takes precedence; flying and swift cadences fill in
        // the gaps between standard waves.
        let kind = if round.is_multiple_of(20) {
            RoundKind::Boss
        } else if round.is_multiple_of(7) {
            RoundKind::Flying
        } else if round.is_multiple_of(5) {
            RoundKind::Swift
        } else {
            RoundKind::Normal
        };

        match kind {
            RoundKind::Normal => Self {
                kind,
                count: base_count,
                health: base_health,
                speed: base_speed,
                flying: false,
            },
            RoundKind::Swift => Self {
                kind,
                count: base_count,
                health: base_health * 0.5,
                speed: base_speed * 1.9,
                flying: false,
            },
            RoundKind::Flying => Self {
                kind,
                count: base_count,
                health: base_health * 0.55,
                speed: base_speed * 0.7,
                flying: true,
            },
            RoundKind::Boss => Self {
                kind,
                count: 1,
                health: base_health * 28.0,
                speed: base_speed * 0.55,
                flying: false,
            },
        }
    }
}

pub fn random_offers(rng: &mut OfferRng) -> [Option<GemKind>; OFFER_COUNT] {
    let mut offers = [None; OFFER_COUNT];
    for offer in &mut offers {
        *offer = Some(GEM_KINDS[rng.next_index(GEM_KINDS.len())]);
    }
    offers
}

pub fn ready_timer(seconds: f32, mode: TimerMode) -> Timer {
    let mut timer = Timer::from_seconds(seconds, mode);
    timer.set_elapsed(Duration::from_secs_f32(seconds));
    timer
}
