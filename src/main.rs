mod board;
mod combat;
mod components;
mod constants;
mod enemy_art;
mod game;
mod gem;
mod gem_visual;
mod grid;
mod input;
mod multiplayer;
mod rng;
mod ui;
mod wave;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode, WindowPlugin};
use board::Board;
use combat::{
    apply_burn, apply_damage_auras, apply_poison, apply_slow_auras, cleanup_effects, reap_enemies,
    tower_attack, update_armor_break, update_enemy_visuals, update_slow, update_stun,
};
use enemy_art::EnemyArt;
use game::Game;
use gem_visual::GemImages;
use input::{
    CameraDrag, handle_chance_buy_clicks, handle_chances_toggle, handle_keep_confirm_clicks,
    handle_offer_clicks, handle_show_range_clicks, handle_speed_clicks, handle_tower_action_clicks,
    pan_and_zoom_camera, place_or_select, select_offer, update_placement_preview,
};
use ui::{
    MultiplayerUiState, handle_escape_menu_buttons, handle_menu_clicks,
    handle_multiplayer_text_input, setup, start_multiplayer_game_from_layout,
    sync_aura_range_rings, sync_upgrade_highlights, toggle_escape_menu, update_chances_panel,
    update_hud, update_join_lobby_screen, update_multiplayer_status_text,
    update_multiplayer_text_inputs, update_offer_visuals, update_round_info,
    update_show_range_button, update_top_bar, update_upgrade_button, update_waiting_lobby_screen,
};
use wave::{animate_enemies, move_enemies, run_wave, update_wave_countdown};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
enum GameSet {
    Input,
    Gameplay,
    Ui,
    Menu,
}

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.045, 0.05, 0.06)))
        .insert_resource(Board::new())
        .insert_resource(Game::new())
        .init_resource::<multiplayer::MultiplayerClient>()
        .init_resource::<MultiplayerUiState>()
        .init_resource::<CameraDrag>()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Geode TD".into(),
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                    ..default()
                }),
        )
        .init_resource::<GemImages>()
        .init_resource::<EnemyArt>()
        .add_systems(Startup, setup)
        .configure_sets(
            Update,
            (
                GameSet::Input,
                GameSet::Gameplay,
                GameSet::Ui,
                GameSet::Menu,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                pan_and_zoom_camera,
                select_offer,
                handle_offer_clicks,
                handle_speed_clicks,
                handle_tower_action_clicks,
                handle_show_range_clicks,
                handle_chances_toggle,
                handle_chance_buy_clicks,
                handle_keep_confirm_clicks,
                place_or_select,
                handle_multiplayer_text_input,
            )
                .chain()
                .in_set(GameSet::Input),
        )
        .add_systems(
            Update,
            (
                update_wave_countdown,
                run_wave,
                apply_poison,
                apply_burn,
                update_armor_break,
                apply_damage_auras,
                tower_attack,
                apply_slow_auras,
                reap_enemies,
                update_slow,
                update_stun,
                move_enemies,
                animate_enemies,
                update_enemy_visuals,
                cleanup_effects,
            )
                .chain()
                .in_set(GameSet::Gameplay),
        )
        .add_systems(
            Update,
            (
                update_offer_visuals,
                update_hud,
                update_top_bar,
                update_round_info,
                update_upgrade_button,
                update_show_range_button,
                update_chances_panel,
                multiplayer::poll_multiplayer_messages,
                update_multiplayer_text_inputs,
                update_multiplayer_status_text,
                update_join_lobby_screen,
                update_waiting_lobby_screen,
                start_multiplayer_game_from_layout,
                sync_upgrade_highlights,
                sync_aura_range_rings,
                update_placement_preview,
            )
                .chain()
                .in_set(GameSet::Ui),
        )
        .add_systems(
            Update,
            (
                handle_escape_menu_buttons,
                handle_menu_clicks,
                toggle_escape_menu,
            )
                .chain()
                .in_set(GameSet::Menu),
        )
        .run();
}
