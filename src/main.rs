mod board;
mod combat;
mod components;
mod constants;
mod game;
mod gem;
mod grid;
mod input;
mod rng;
mod ui;
mod wave;

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode, WindowPlugin};
use board::Board;
use combat::{
    apply_poison, cleanup_effects, reap_enemies, tower_attack, update_enemy_visuals, update_slow,
};
use game::Game;
use input::{place_or_select, select_offer};
use ui::{
    handle_menu_clicks, setup, toggle_escape_menu, update_hud, update_offer_visuals, update_top_bar,
};
use wave::{move_enemies, run_wave, update_wave_countdown};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.045, 0.05, 0.06)))
        .insert_resource(Board::new())
        .insert_resource(Game::new())
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Geode TD".into(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                select_offer,
                place_or_select,
                update_wave_countdown,
                run_wave,
                apply_poison,
                tower_attack,
                reap_enemies,
                update_slow,
                move_enemies,
                update_enemy_visuals,
                cleanup_effects,
                update_offer_visuals,
                update_hud,
                update_top_bar,
                handle_menu_clicks,
                toggle_escape_menu,
            )
                .chain(),
        )
        .run();
}
