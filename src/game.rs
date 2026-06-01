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
    pub selected_offer: usize,
    pub pending_enemies: u32,
    pub spawn_timer: Timer,
    pub countdown_timer: Timer,
    pub rng: OfferRng,
    pub message: String,
    pub selected_tower: Option<Entity>,
    pub upgrade_source: Option<Entity>,
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
            selected_offer: 0,
            pending_enemies: 0,
            spawn_timer: ready_timer(0.65, TimerMode::Repeating),
            countdown_timer: Timer::from_seconds(WAVE_COUNTDOWN_SECONDS, TimerMode::Once),
            rng,
            message: "Pick one of five chipped gems and place it.".to_string(),
            selected_tower: None,
            upgrade_source: None,
        }
    }

    pub fn selected_gem(&self) -> Option<GemKind> {
        self.offers[self.selected_offer]
    }

    pub fn refresh_offers(&mut self) {
        self.offers = random_offers(&mut self.rng);
        self.selected_offer = 0;
    }

    pub fn clear_offers(&mut self) {
        self.offers = [None; OFFER_COUNT];
    }

    pub fn begin_countdown(&mut self, gem: GemKind) {
        self.phase = Phase::Countdown;
        self.clear_offers();
        self.selected_tower = None;
        self.upgrade_source = None;
        self.countdown_timer = Timer::from_seconds(WAVE_COUNTDOWN_SECONDS, TimerMode::Once);
        self.message = format!(
            "Placed a Chipped {}. Wave starts in {:.0} seconds.",
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
        self.message = format!("Round {}: pick one of five chipped gems.", self.round);
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
        self.message = format!("{} mode: pick one of five chipped gems.", mode.name());
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
