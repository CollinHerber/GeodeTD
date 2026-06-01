use bevy::prelude::*;
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
    }

    pub fn clear_offers(&mut self) {
        self.offers = [None; OFFER_COUNT];
        self.selected_offer = None;
        self.placed_starters = [None; OFFER_COUNT];
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
        self.pending_enemies = 5 + self.round * 2;
        self.spawn_timer = ready_timer(0.65, TimerMode::Repeating);
        self.message = format!("Wave {} is moving through the checkpoints.", self.round);
    }

    pub fn begin_build_round(&mut self) {
        self.phase = Phase::Build;
        self.round += 1;
        self.refresh_offers();
        self.upgrade_source = None;
        self.message = format!(
            "Round {}: place all five chipped gems, then choose one to keep.",
            self.round
        );
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
