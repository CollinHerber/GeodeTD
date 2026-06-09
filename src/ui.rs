use bevy::app::AppExit;
use bevy::ecs::query::QueryFilter;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

use crate::board::Board;
use crate::components::{
    AuraRangeSegment, ChanceBuyButton, ChanceBuyText, ChanceRowText, ChancesButton,
    ChancesHeaderText, ChancesPanel, CheckpointMarker, ConfirmKeepButton, DisplayNameScreen, Enemy,
    EscapeMenu, EscapeMenuAction, EscapeMenuButton, EscapeMenuInfo, GameWorld, HomeScreen,
    HostLobbyScreen, HowToPlayScreen, HudText, JoinLobbyScreen, LobbyRowButton, MenuAction,
    MenuButton, ModeSelectScreen, MultiplayerDynamic, MultiplayerMenuScreen, MultiplayerStatusText,
    OfferButton, OfferLabel, OfferSelectionGlow, OfferVisual, PathMarker, PlayTypeScreen,
    RoundInfoBody, RoundInfoTitle, SelectionMenu, SettingsScreen, ShowRangeButton,
    ShowRangeButtonText, SpeedButton, SpeedText, StarterCandidate, TextInputDisplay,
    TextInputField, TopBarText, Tower, UpgradeButton, UpgradeButtonText, UpgradeHighlight,
    WaitingLobbyScreen,
};
use crate::constants::{CELL_SIZE, OFFER_COUNT};
use crate::game::{AppScreen, Game, GameMode, Phase, RoundPlan, UpgradeChances, tier_grade};
use crate::gem::{
    GRADE_LADDER, GemEffect, GemGrade, GemKind, SpecialRecipe, special_recipe_for_source,
};
use crate::gem_visual::GemImages;
use crate::grid::{GridPos, grid_to_world};
use crate::multiplayer::{MultiplayerClient, NameIntent};

#[derive(Resource, Default)]
pub struct MultiplayerUiState {
    join_revision: u64,
    lobby_revision: u64,
}

type MenuScreenFilter = Or<(
    With<HomeScreen>,
    With<PlayTypeScreen>,
    With<ModeSelectScreen>,
    With<MultiplayerMenuScreen>,
    With<DisplayNameScreen>,
    With<HostLobbyScreen>,
    With<JoinLobbyScreen>,
    With<WaitingLobbyScreen>,
    With<HowToPlayScreen>,
    With<SettingsScreen>,
)>;

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    spawn_home_screen(&mut commands);
}

#[allow(clippy::too_many_arguments)]
pub fn handle_menu_clicks(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    mut multiplayer: ResMut<MultiplayerClient>,
    mut app_exit: MessageWriter<AppExit>,
    buttons_query: Query<&MenuButton>,
    lobby_buttons: Query<&LobbyRowButton>,
    menu_entities: Query<Entity, MenuScreenFilter>,
    game_entities: Query<Entity, With<GameWorld>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(world_pos) = cursor_world_position(&windows, &camera) else {
        return;
    };

    if game.screen == AppScreen::JoinLobby
        && let Some(lobby_id) = lobby_buttons
            .iter()
            .find(|button| point_in_rect(world_pos, button.center, button.size))
            .map(|button| button.lobby_id.clone())
    {
        despawn_entities(&mut commands, &menu_entities);
        multiplayer.join_lobby(lobby_id);
        spawn_waiting_lobby_screen(&mut commands, &multiplayer);
        game.screen = AppScreen::WaitingLobby;
        return;
    }

    let Some(action) = buttons_query
        .iter()
        .find(|button| point_in_rect(world_pos, button.center, button.size))
        .map(|button| button.action)
    else {
        return;
    };

    match action {
        MenuAction::Play => {
            despawn_entities(&mut commands, &menu_entities);
            spawn_play_type_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::PlayType;
        }
        MenuAction::SinglePlayer => {
            despawn_entities(&mut commands, &menu_entities);
            spawn_mode_select_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::ModeSelect;
        }
        MenuAction::Multiplayer => {
            despawn_entities(&mut commands, &menu_entities);
            spawn_multiplayer_menu_screen(&mut commands);
            multiplayer.ensure_connected();
            multiplayer.request_lobbies();
            game.paused = false;
            game.screen = AppScreen::MultiplayerMenu;
        }
        MenuAction::Host => {
            despawn_entities(&mut commands, &menu_entities);
            if multiplayer.has_display_name() {
                spawn_host_lobby_screen(&mut commands, &multiplayer);
                game.screen = AppScreen::HostLobby;
            } else {
                multiplayer.pending_name_intent = Some(NameIntent::Host);
                spawn_display_name_screen(&mut commands, &multiplayer);
                game.screen = AppScreen::DisplayName;
            }
        }
        MenuAction::Join => {
            despawn_entities(&mut commands, &menu_entities);
            if multiplayer.has_display_name() {
                multiplayer.ensure_connected();
                multiplayer.request_lobbies();
                spawn_join_lobby_screen(&mut commands, &multiplayer);
                game.screen = AppScreen::JoinLobby;
            } else {
                multiplayer.pending_name_intent = Some(NameIntent::Join);
                spawn_display_name_screen(&mut commands, &multiplayer);
                game.screen = AppScreen::DisplayName;
            }
        }
        MenuAction::SubmitDisplayName => {
            multiplayer.save_display_name_from_input();
            despawn_entities(&mut commands, &menu_entities);
            match multiplayer.pending_name_intent.take() {
                Some(NameIntent::Host) => {
                    spawn_host_lobby_screen(&mut commands, &multiplayer);
                    game.screen = AppScreen::HostLobby;
                }
                Some(NameIntent::Join) => {
                    multiplayer.ensure_connected();
                    multiplayer.request_lobbies();
                    spawn_join_lobby_screen(&mut commands, &multiplayer);
                    game.screen = AppScreen::JoinLobby;
                }
                None => {
                    spawn_multiplayer_menu_screen(&mut commands);
                    game.screen = AppScreen::MultiplayerMenu;
                }
            }
        }
        MenuAction::CreateLobby => {
            despawn_entities(&mut commands, &menu_entities);
            multiplayer.ensure_connected();
            multiplayer.create_lobby();
            spawn_waiting_lobby_screen(&mut commands, &multiplayer);
            game.screen = AppScreen::WaitingLobby;
        }
        MenuAction::StartMultiplayerGame => {
            multiplayer.start_game();
        }
        MenuAction::LeaveLobby => {
            multiplayer.leave_lobby();
            despawn_entities(&mut commands, &menu_entities);
            spawn_multiplayer_menu_screen(&mut commands);
            game.screen = AppScreen::MultiplayerMenu;
        }
        MenuAction::Standard => {
            despawn_entities(&mut commands, &menu_entities);
            despawn_all(&mut commands, &game_entities);
            board.reset_for_mode(GameMode::Standard);
            game.reset_for_mode(GameMode::Standard);
            spawn_game_scene(&mut commands, &board, game.show_path_overlay);
            game.screen = AppScreen::Playing;
        }
        MenuAction::Random => {
            despawn_entities(&mut commands, &menu_entities);
            despawn_all(&mut commands, &game_entities);
            board.reset_for_mode(GameMode::Random);
            game.reset_for_mode(GameMode::Random);
            spawn_game_scene(&mut commands, &board, game.show_path_overlay);
            game.screen = AppScreen::Playing;
        }
        MenuAction::HowToPlay => {
            despawn_entities(&mut commands, &menu_entities);
            spawn_how_to_play_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::HowToPlay;
        }
        MenuAction::Settings => {
            despawn_entities(&mut commands, &menu_entities);
            spawn_settings_screen(&mut commands, game.show_path_overlay);
            game.paused = false;
            game.screen = AppScreen::Settings;
        }
        MenuAction::TogglePathOverlay => {
            game.show_path_overlay = !game.show_path_overlay;
            despawn_entities(&mut commands, &menu_entities);
            spawn_settings_screen(&mut commands, game.show_path_overlay);
            game.paused = false;
            game.screen = AppScreen::Settings;
        }
        MenuAction::Quit => {
            app_exit.write(AppExit::Success);
        }
        MenuAction::Home => {
            multiplayer.leave_lobby();
            despawn_entities(&mut commands, &menu_entities);
            despawn_all(&mut commands, &game_entities);
            spawn_home_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::Home;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_escape_menu_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &EscapeMenuButton), Changed<Interaction>>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    game_entities: Query<Entity, With<GameWorld>>,
    home_entities: Query<Entity, With<HomeScreen>>,
    mode_entities: Query<Entity, With<ModeSelectScreen>>,
    how_to_play_entities: Query<Entity, With<HowToPlayScreen>>,
    settings_entities: Query<Entity, With<SettingsScreen>>,
    escape_info: Query<Entity, With<EscapeMenuInfo>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.action {
            EscapeMenuAction::Reset => {
                despawn_all(&mut commands, &game_entities);
                let mode = game.mode;
                board.reset_for_mode(mode);
                game.reset_for_mode(mode);
                reset_camera(&mut camera);
                spawn_game_scene(&mut commands, &board, game.show_path_overlay);
            }
            EscapeMenuAction::Home => {
                despawn_all(&mut commands, &home_entities);
                despawn_all(&mut commands, &mode_entities);
                despawn_all(&mut commands, &how_to_play_entities);
                despawn_all(&mut commands, &settings_entities);
                despawn_all(&mut commands, &game_entities);
                game.paused = false;
                game.screen = AppScreen::Home;
                reset_camera(&mut camera);
                spawn_home_screen(&mut commands);
            }
            EscapeMenuAction::HowToPlay => {
                if escape_info.iter().next().is_some() {
                    despawn_all(&mut commands, &escape_info);
                } else {
                    spawn_escape_how_to_play(&mut commands);
                }
            }
        }
    }
}

fn spawn_game_scene(commands: &mut Commands, board: &Board, show_path_overlay: bool) {
    spawn_board_tiles(commands, board);
    spawn_path_markers(commands, &board.path, show_path_overlay);
    spawn_checkpoint_markers(commands, &board.checkpoints);
    spawn_play_ui(commands);
    spawn_offer_bar(commands);
    spawn_round_info_panel(commands);
}

/// Persistent left-side panel describing the current round: its kind, head count,
/// and per-enemy health and speed. Filled in each frame by [`update_round_info`].
fn spawn_round_info_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(160),
                left: px(16),
                width: px(236),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.07, 0.085, 0.94)),
            GameWorld,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.95, 0.95)),
                RoundInfoTitle,
            ));
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.84, 0.86)),
                RoundInfoBody,
            ));
        });
}

fn spawn_play_ui(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: px(52),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.04, 0.045, 0.96)),
            GameWorld,
        ))
        .with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(16),
                    width: px(116),
                    height: px(34),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.20, 0.26, 0.24)),
                ChancesButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("Chances"),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.97, 0.92)),
                ));
            });
            bar.spawn((
                Text::new(""),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(0.93, 0.95, 0.96)),
                TopBarText,
            ));
            bar.spawn((
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    right: px(16),
                    width: px(82),
                    height: px(34),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.23, 0.24)),
                SpeedButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new("1x"),
                    TextFont {
                        font_size: 18.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.95, 0.97, 0.92)),
                    SpeedText,
                ));
            });
        });
}

/// Builds the upgrading-chances popup (header + one row per upgradable tier).
/// Text and button colors are filled in each frame by [`update_chances_panel`].
pub fn spawn_chances_panel(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(56),
                left: px(16),
                width: px(300),
                padding: UiRect::all(px(12)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.07, 0.085, 0.98)),
            ChancesPanel,
            GameWorld,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(""),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.95, 0.95)),
                ChancesHeaderText,
            ));
            for tier in 0..UpgradeChances::TIERS {
                panel
                    .spawn(Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceBetween,
                        column_gap: px(10),
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new(""),
                            TextFont {
                                font_size: 15.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.82, 0.86, 0.88)),
                            ChanceRowText { tier },
                        ));
                        row.spawn((
                            Button,
                            Node {
                                width: px(110),
                                height: px(30),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.18, 0.30, 0.22)),
                            ChanceBuyButton { tier },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new(""),
                                TextFont {
                                    font_size: 13.0,
                                    ..default()
                                },
                                TextColor(Color::srgb(0.95, 0.97, 0.92)),
                                ChanceBuyText { tier },
                            ));
                        });
                    });
            }
        });
}

/// While the chances panel is open, refresh each tier's odds, the next cost, and
/// whether each buy button is affordable.
#[allow(clippy::type_complexity)]
pub fn update_chances_panel(
    game: Res<Game>,
    mut header: Query<
        &mut Text,
        (
            With<ChancesHeaderText>,
            Without<ChanceRowText>,
            Without<ChanceBuyText>,
        ),
    >,
    mut rows: Query<
        (&ChanceRowText, &mut Text),
        (Without<ChancesHeaderText>, Without<ChanceBuyText>),
    >,
    mut buy_texts: Query<
        (&ChanceBuyText, &mut Text),
        (Without<ChancesHeaderText>, Without<ChanceRowText>),
    >,
    mut buy_buttons: Query<(&ChanceBuyButton, &mut BackgroundColor)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let cost = game.chances.next_cost();

    if let Ok(mut text) = header.single_mut() {
        **text = format!(
            "Upgrading Chances\nGold {}    Chipped {}%    Next: {}",
            game.coins,
            game.chances.chipped_pct(),
            cost
        );
    }

    for (row, mut text) in &mut rows {
        **text = format!(
            "{} {}%",
            tier_grade(row.tier).name(),
            game.chances.pct(row.tier)
        );
    }

    for (buy, mut text) in &mut buy_texts {
        **text = if game.chances.at_cap(buy.tier) {
            "MAX".to_string()
        } else {
            format!("+10%  ({})", cost)
        };
    }

    for (buy, mut color) in &mut buy_buttons {
        let affordable = !game.chances.at_cap(buy.tier) && game.coins >= cost;
        color.0 = if affordable {
            Color::srgb(0.18, 0.30, 0.22)
        } else {
            Color::srgb(0.13, 0.14, 0.15)
        };
    }
}

pub fn toggle_escape_menu(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    menu_items: Query<Entity, With<EscapeMenu>>,
) {
    if game.screen != AppScreen::Playing || !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    if menu_items.iter().next().is_some() {
        despawn_all(&mut commands, &menu_items);
        game.paused = false;
    } else {
        spawn_escape_menu(&mut commands);
        game.paused = true;
    }
}

pub fn refresh_path_markers(
    commands: &mut Commands,
    path_markers: &Query<Entity, With<PathMarker>>,
    path: &[GridPos],
    show_path_overlay: bool,
) {
    for entity in path_markers.iter() {
        commands.entity(entity).despawn();
    }
    spawn_path_markers(commands, path, show_path_overlay);
}

pub fn clear_selection_menu(
    commands: &mut Commands,
    menu_items: &Query<Entity, With<SelectionMenu>>,
) {
    for entity in menu_items.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn spawn_selection_menu(commands: &mut Commands, tower: &Tower) {
    let stats = stats_text(
        tower.attack_damage(),
        tower.range,
        tower.cooldown.duration().as_secs_f32(),
        tower.effect(),
    );

    spawn_info_panel(commands, &tower.display_name(), &stats);
    spawn_action_bar(
        commands,
        &tower.action_label(),
        tower.range_indicator_radius().is_some(),
    );
}

/// Read-only stat panel for a gem offer (before placement). Shows the same stats
/// as a placed tower at Chipped grade, but without the upgrade button.
pub fn spawn_gem_info(commands: &mut Commands, gem: GemKind, grade: GemGrade) {
    let stats = gem.chipped_stats();
    let body = stats_text(
        stats.damage * grade.damage_multiplier(),
        stats.range,
        stats.cooldown,
        gem.effect(grade),
    );

    spawn_info_panel(commands, &format!("{} {}", grade.name(), gem.name()), &body);
}

/// Shows the chosen starter's stats alongside a Confirm button that commits the
/// keep once all five are placed.
pub fn spawn_keep_confirm(commands: &mut Commands, gem: GemKind, grade: GemGrade) {
    spawn_gem_info(commands, gem, grade);
    spawn_confirm_bar(
        commands,
        &format!("Confirm: keep {} {}", grade.name(), gem.name()),
    );
}

fn spawn_confirm_bar(commands: &mut Commands, action: &str) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                width: percent(100),
                height: px(86),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.04, 0.045, 0.95)),
            SelectionMenu,
            GameWorld,
        ))
        .with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    width: px(220),
                    height: px(46),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.16, 0.32, 0.21)),
                ConfirmKeepButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(action.to_string()),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.93, 0.97, 0.92)),
                ));
            });
        });
}

fn spawn_info_panel(commands: &mut Commands, title: &str, body: &str) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(68),
                right: px(16),
                width: px(276),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.09, 0.10, 0.94)),
            SelectionMenu,
            GameWorld,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new(title.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.95, 0.95)),
            ));
            panel.spawn((
                Text::new(body.to_string()),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(Color::srgb(0.80, 0.84, 0.86)),
            ));
        });
}

fn spawn_action_bar(commands: &mut Commands, action: &str, show_range_button: bool) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                width: percent(100),
                height: px(86),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.04, 0.045, 0.95)),
            SelectionMenu,
            GameWorld,
        ))
        .with_children(|bar| {
            bar.spawn((
                Button,
                Node {
                    width: px(190),
                    height: px(46),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.23, 0.24)),
                UpgradeButton,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(action.to_string()),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.96, 0.96, 0.92)),
                    UpgradeButtonText,
                ));
            });

            if show_range_button {
                bar.spawn((
                    Button,
                    Node {
                        width: px(150),
                        height: px(46),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.27, 0.29)),
                    ShowRangeButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Show Range"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.92, 0.98, 0.98)),
                        ShowRangeButtonText,
                    ));
                });
            }
        });
}

fn stats_text(damage: f32, range: f32, cooldown: f32, effect: GemEffect) -> String {
    let fire_rate = if cooldown > 0.0 { 1.0 / cooldown } else { 0.0 };
    format!(
        "Damage: {}\nRange: {:.0}\nFire rate: {:.2}/s\n{}",
        damage_text(damage),
        range,
        fire_rate,
        effect.describe()
    )
}

fn damage_text(damage: f32) -> String {
    if (damage - damage.round()).abs() < 0.05 {
        format!("{damage:.0}")
    } else {
        format!("{damage:.1}")
    }
}

pub fn update_top_bar(
    game: Res<Game>,
    mut bar: Query<&mut Text, With<TopBarText>>,
    mut speed: Query<&mut Text, (With<SpeedText>, Without<TopBarText>)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let Ok(mut text) = bar.single_mut() else {
        return;
    };

    **text = format!(
        "Wave {}        Lives {}        Gold {}",
        game.round,
        game.lives.max(0),
        game.coins
    );

    if let Ok(mut speed) = speed.single_mut() {
        **speed = format!("{}x", game.speed);
    }
}

pub fn update_round_info(
    game: Res<Game>,
    enemies: Query<(), With<Enemy>>,
    mut title: Query<(&mut Text, &mut TextColor), With<RoundInfoTitle>>,
    mut body: Query<&mut Text, (With<RoundInfoBody>, Without<RoundInfoTitle>)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let plan = RoundPlan::for_round(game.round);

    if let Ok((mut text, mut color)) = title.single_mut() {
        **text = format!("Round {} — {}", game.round, plan.kind.label());
        color.0 = plan.kind.accent();
    }

    let Ok(mut text) = body.single_mut() else {
        return;
    };

    // During the wave, show how many are still unspawned or alive so the player can
    // gauge how much of the round is left.
    let progress = if game.phase == Phase::Wave {
        let remaining = game.pending_enemies as usize + enemies.iter().count();
        format!("\nRemaining: {} / {}", remaining, plan.count)
    } else {
        String::new()
    };

    **text = format!(
        "{}\n\nEnemies: {}\nHealth: {:.0} each\nSpeed: {:.0}\nMovement: {}{}",
        plan.kind.description(),
        plan.count,
        plan.health,
        plan.speed,
        if plan.flying { "Flying" } else { "Grounded" },
        progress,
    );
}

pub fn handle_multiplayer_text_input(
    game: Res<Game>,
    mut multiplayer: ResMut<MultiplayerClient>,
    mut keyboard: MessageReader<KeyboardInput>,
) {
    let active = match game.screen {
        AppScreen::DisplayName => Some(TextInputField::DisplayName),
        AppScreen::HostLobby => Some(TextInputField::LobbyName),
        _ => None,
    };
    let Some(active) = active else {
        return;
    };

    let input = match active {
        TextInputField::DisplayName => &mut multiplayer.display_name_input,
        TextInputField::LobbyName => &mut multiplayer.lobby_name_input,
    };

    for event in keyboard.read() {
        if event.state != ButtonState::Pressed {
            continue;
        }

        match event.key_code {
            KeyCode::Backspace => {
                input.pop();
            }
            KeyCode::Enter => {}
            _ => {
                let Some(text) = &event.text else {
                    continue;
                };
                for character in text.chars() {
                    if input.chars().count() >= max_input_chars(active) {
                        break;
                    }
                    if character.is_control() {
                        continue;
                    }
                    input.push(character);
                }
            }
        }
    }
}

pub fn update_multiplayer_text_inputs(
    multiplayer: Res<MultiplayerClient>,
    mut inputs: Query<(&TextInputDisplay, &mut Text2d)>,
) {
    for (display, mut text) in &mut inputs {
        text.0 = match display.field {
            TextInputField::DisplayName => text_input_value(&multiplayer.display_name_input),
            TextInputField::LobbyName => text_input_value(&multiplayer.lobby_name_input),
        };
    }
}

pub fn update_multiplayer_status_text(
    multiplayer: Res<MultiplayerClient>,
    mut statuses: Query<&mut Text2d, With<MultiplayerStatusText>>,
) {
    for mut text in &mut statuses {
        text.0 = multiplayer.status.clone();
    }
}

pub fn update_join_lobby_screen(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    mut multiplayer: ResMut<MultiplayerClient>,
    mut ui_state: ResMut<MultiplayerUiState>,
    dynamic_entities: Query<Entity, With<MultiplayerDynamic>>,
) {
    if game.screen != AppScreen::JoinLobby {
        return;
    }

    multiplayer.tick_join_refresh(&time);

    if ui_state.join_revision == multiplayer.list_revision {
        return;
    }

    despawn_all(&mut commands, &dynamic_entities);
    spawn_lobby_rows(&mut commands, &multiplayer, JoinLobbyScreen);
    ui_state.join_revision = multiplayer.list_revision;
}

pub fn update_waiting_lobby_screen(
    mut commands: Commands,
    game: Res<Game>,
    multiplayer: Res<MultiplayerClient>,
    mut ui_state: ResMut<MultiplayerUiState>,
    dynamic_entities: Query<Entity, With<MultiplayerDynamic>>,
) {
    if game.screen != AppScreen::WaitingLobby {
        return;
    }

    if ui_state.lobby_revision == multiplayer.lobby_revision {
        return;
    }

    despawn_all(&mut commands, &dynamic_entities);
    spawn_waiting_lobby_dynamic(&mut commands, &multiplayer);
    ui_state.lobby_revision = multiplayer.lobby_revision;
}

#[allow(clippy::too_many_arguments)]
pub fn start_multiplayer_game_from_layout(
    mut commands: Commands,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    mut multiplayer: ResMut<MultiplayerClient>,
    menu_entities: Query<Entity, MenuScreenFilter>,
    game_entities: Query<Entity, With<GameWorld>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    let Some(layout) = multiplayer.pending_layout.take() else {
        return;
    };

    despawn_entities(&mut commands, &menu_entities);
    despawn_all(&mut commands, &game_entities);

    let mode = match layout.mode {
        geode_td_shared::GameMode::Standard => GameMode::Standard,
        geode_td_shared::GameMode::Random => GameMode::Random,
    };
    board.reset_for_shared_layout(&layout);
    game.reset_for_mode(mode);
    game.message = format!(
        "Multiplayer {} mode: place all five chipped gems, then choose one to keep.",
        mode.name()
    );
    reset_camera(&mut camera);
    spawn_game_scene(&mut commands, &board, game.show_path_overlay);
    game.screen = AppScreen::Playing;
}

pub fn update_offer_visuals(
    game: Res<Game>,
    time: Res<Time>,
    gem_images: Res<GemImages>,
    mut offer_buttons: Query<(&OfferButton, &mut BackgroundColor), Without<OfferSelectionGlow>>,
    mut offer_sprites: Query<
        (&OfferVisual, &mut ImageNode, &mut Node),
        Without<OfferSelectionGlow>,
    >,
    mut offer_glows: Query<
        (&OfferSelectionGlow, &mut BackgroundColor, &mut Node),
        Without<OfferButton>,
    >,
    mut offer_labels: Query<(&OfferLabel, &mut Text, &mut TextColor), Without<OfferVisual>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (offer, mut color) in &mut offer_buttons {
        let selected = game.selected_offer == Some(offer.index) && game.phase == Phase::Build;
        color.0 = if selected {
            Color::srgb(0.18, 0.32, 0.31)
        } else if game.placed_starters[offer.index].is_some() {
            Color::srgba(0.11, 0.13, 0.13, 0.94)
        } else {
            Color::srgba(0.08, 0.09, 0.10, 0.94)
        };
    }

    let pulse = (time.elapsed_secs() * 5.4).sin() * 0.5 + 0.5;
    for (glow, mut color, mut node) in &mut offer_glows {
        let selected = game.selected_offer == Some(glow.index) && game.phase == Phase::Build;
        if selected {
            let size = 58.0 + pulse * 8.0;
            node.width = px(size);
            node.height = px(size);
            color.0 = Color::srgba(0.50, 0.94, 0.86, 0.18 + pulse * 0.24);
        } else {
            node.width = px(0);
            node.height = px(0);
            color.0 = Color::NONE;
        }
    }

    for (offer, mut image, mut node) in &mut offer_sprites {
        let selected = game.selected_offer == Some(offer.index) && game.phase == Phase::Build;
        let size = if selected { 50.0 } else { 34.0 };
        node.width = px(size);
        node.height = px(size);

        image.image = match game.offers[offer.index] {
            Some(gem) => gem_images.handle(gem, game.offer_grades[offer.index]),
            None => gem_images.empty(),
        };
        image.color = Color::WHITE;
    }

    for (label, mut text, mut color) in &mut offer_labels {
        **text = match (game.offers[label.index], game.placed_starters[label.index]) {
            (Some(gem), _) => format!("{}\n{}", game.offer_grades[label.index].name(), gem.name()),
            (None, Some(_)) => "Placed".to_string(),
            (None, None) => "--".to_string(),
        };
        color.0 =
            if game.offers[label.index].is_some() || game.placed_starters[label.index].is_some() {
                Color::srgb(0.93, 0.94, 0.95)
            } else {
                Color::srgb(0.45, 0.47, 0.48)
            };
    }
}

pub fn update_hud(game: Res<Game>, board: Res<Board>, mut hud: Query<&mut Text2d, With<HudText>>) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let Ok(mut text) = hud.single_mut() else {
        return;
    };

    let phase = match game.phase {
        Phase::Build => "Build",
        Phase::Countdown => "Countdown",
        Phase::Wave => "Wave",
    };
    let countdown = if game.phase == Phase::Countdown {
        format!(
            "      Starts in: {:.1}",
            game.countdown_timer.remaining_secs().max(0.0)
        )
    } else {
        String::new()
    };
    let starter_prompt = if game.phase == Phase::Build && game.keep_candidate.is_some() {
        "      Press Confirm to keep, or pick another starter"
    } else if game.phase == Phase::Build && game.all_starters_placed() {
        "      Click a starter to select, then Confirm (Ctrl+click = instant)"
    } else if game.phase == Phase::Build {
        "      Place all five starters"
    } else {
        ""
    };
    let prompt = if game.upgrade_source.is_some() {
        "      Upgrade: click a matching duplicate to sacrifice"
    } else {
        starter_prompt
    };

    text.0 = format!(
        "Mode: {}      Round: {}      Phase: {}{}\nLives: {}      Gold: {}      Path: {}\n{}{}\nGrades: {}",
        game.mode.name(),
        game.round,
        phase,
        countdown,
        game.lives.max(0),
        game.coins,
        board.path.len(),
        game.message,
        prompt,
        grade_ladder_text()
    );
}

/// Grays out the "Upgrade to X" button unless the selected tower can actually be
/// upgraded right now: it's a build round, the gem isn't already Perfect, and a
/// matching placed (non-starter) duplicate exists to sacrifice.
pub fn update_upgrade_button(
    game: Res<Game>,
    towers: Query<(Entity, &Tower)>,
    starters: Query<&StarterCandidate>,
    mut buttons: Query<&mut BackgroundColor, With<UpgradeButton>>,
    mut labels: Query<&mut TextColor, With<UpgradeButtonText>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let enabled = upgrade_is_available(&game, &towers, &starters);

    for mut color in &mut buttons {
        color.0 = if enabled {
            Color::srgb(0.18, 0.23, 0.24)
        } else {
            Color::srgb(0.12, 0.13, 0.14)
        };
    }
    for mut color in &mut labels {
        color.0 = if enabled {
            Color::srgb(0.96, 0.96, 0.92)
        } else {
            Color::srgb(0.46, 0.48, 0.49)
        };
    }
}

pub fn update_show_range_button(
    game: Res<Game>,
    mut buttons: Query<&mut BackgroundColor, With<ShowRangeButton>>,
    mut labels: Query<&mut Text, With<ShowRangeButtonText>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    let shown = game
        .selected_tower
        .is_some_and(|tower| game.shown_aura_ranges.contains(&tower));

    for mut color in &mut buttons {
        color.0 = if shown {
            Color::srgb(0.12, 0.36, 0.34)
        } else {
            Color::srgb(0.15, 0.27, 0.29)
        };
    }
    for mut text in &mut labels {
        **text = if shown {
            "Hide Range".to_string()
        } else {
            "Show Range".to_string()
        };
    }
}

fn upgrade_is_available(
    game: &Game,
    towers: &Query<(Entity, &Tower)>,
    starters: &Query<&StarterCandidate>,
) -> bool {
    let Some(source) = game.selected_tower else {
        return false;
    };
    let Ok((_, source_tower)) = towers.get(source) else {
        return false;
    };

    if let Some(special) = source_tower.special
        && special.upgrade().is_some()
    {
        return special
            .upgrade_cost()
            .is_some_and(|cost| game.coins >= cost);
    }

    if let Some(recipe) = special_recipe_for_source(source_tower.gem, source_tower.grade) {
        return special_recipe_is_available(source, recipe, towers, starters);
    }

    if !source_tower.can_regular_upgrade() {
        return false;
    }
    towers.iter().any(|(entity, tower)| {
        entity != source
            && starters.get(entity).is_err()
            && tower.is_basic(source_tower.gem, source_tower.grade)
    })
}

fn special_recipe_is_available(
    source: Entity,
    recipe: SpecialRecipe,
    towers: &Query<(Entity, &Tower)>,
    starters: &Query<&StarterCandidate>,
) -> bool {
    recipe.components.iter().all(|(gem, grade)| {
        towers.iter().any(|(entity, tower)| {
            entity != source && starters.get(entity).is_err() && tower.is_basic(*gem, *grade)
        })
    })
}

/// Draws halos behind relevant towers: gold behind every tower that can be
/// sacrificed for an in-progress upgrade, green behind the starter the player has
/// chosen (but not yet confirmed) to keep, or cyan behind the tower whose stats
/// are currently being inspected. Rebuilt each frame.
pub fn sync_upgrade_highlights(
    mut commands: Commands,
    game: Res<Game>,
    towers: Query<(Entity, &Transform, &Tower, Option<&StarterCandidate>)>,
    highlights: Query<Entity, With<UpgradeHighlight>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for entity in &highlights {
        commands.entity(entity).despawn();
    }

    let spawn_halo = |commands: &mut Commands, position: Vec2, color: Color| {
        commands.spawn((
            Sprite::from_color(color, Vec2::splat(CELL_SIZE * 0.94)),
            Transform::from_translation(position.extend(4.5)),
            UpgradeHighlight,
            GameWorld,
        ));
    };

    if let Some(source) = game.upgrade_source {
        let Ok((_, _, source_tower, _)) = towers.get(source) else {
            return;
        };

        for (entity, transform, tower, starter) in &towers {
            let eligible = entity != source
                && starter.is_none()
                && tower.is_basic(source_tower.gem, source_tower.grade);
            if eligible {
                spawn_halo(
                    &mut commands,
                    transform.translation.truncate(),
                    Color::srgba(1.0, 0.9, 0.32, 0.45),
                );
            }
        }
    } else if let Some(candidate) = game.keep_candidate
        && let Ok((_, transform, _, _)) = towers.get(candidate)
    {
        spawn_halo(
            &mut commands,
            transform.translation.truncate(),
            Color::srgba(0.36, 0.92, 0.46, 0.5),
        );
    } else if let Some(selected) = game.selected_tower
        && let Ok((_, transform, _, _)) = towers.get(selected)
    {
        // Cyan halo marks which placed tower the stat panel is describing.
        spawn_halo(
            &mut commands,
            transform.translation.truncate(),
            Color::srgba(0.42, 0.80, 1.0, 0.42),
        );
    }
}

pub fn sync_aura_range_rings(
    mut commands: Commands,
    time: Res<Time>,
    game: Res<Game>,
    towers: Query<(Entity, &Transform, &Tower)>,
    rings: Query<Entity, With<AuraRangeSegment>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for entity in &rings {
        commands.entity(entity).despawn();
    }

    const SEGMENTS: usize = 72;
    let pulse = (time.elapsed_secs() * 2.8).sin() * 0.5 + 0.5;

    for (entity, transform, tower) in &towers {
        if !game.shown_aura_ranges.contains(&entity) {
            continue;
        }

        let Some(radius) = tower.range_indicator_radius() else {
            continue;
        };

        let center = transform.translation.truncate();
        let rgb = tower.srgb();
        let radius = radius + pulse * 4.0;
        let segment_length = std::f32::consts::TAU * radius / SEGMENTS as f32 * 0.62;
        let thickness = 4.0 + pulse * 2.0;
        let alpha = 0.20 + pulse * 0.18;

        for index in 0..SEGMENTS {
            let angle = index as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            let direction = Vec2::new(angle.cos(), angle.sin());
            commands.spawn((
                Sprite::from_color(
                    Color::srgba(rgb[0], rgb[1], rgb[2], alpha),
                    Vec2::new(segment_length, thickness),
                ),
                Transform::from_translation((center + direction * radius).extend(4.2))
                    .with_rotation(Quat::from_rotation_z(angle + std::f32::consts::FRAC_PI_2)),
                AuraRangeSegment,
                GameWorld,
            ));
        }
    }
}

pub fn tower_sprite_size(grade: GemGrade) -> Vec2 {
    Vec2::splat(CELL_SIZE * 0.58 * grade.size_multiplier())
}

fn spawn_board_tiles(commands: &mut Commands, board: &Board) {
    let protected = board.protected_cells();

    for row in 0..crate::constants::GRID_ROWS {
        for col in 0..crate::constants::GRID_COLUMNS {
            let pos = GridPos::new(col, row);
            let color = if pos == board.start {
                Color::srgb(0.10, 0.38, 0.18)
            } else if pos == board.finish {
                Color::srgb(0.42, 0.13, 0.14)
            } else if protected.contains(&pos) {
                Color::srgb(0.22, 0.18, 0.08)
            } else if (row + col) % 2 == 0 {
                Color::srgb(0.13, 0.15, 0.16)
            } else {
                Color::srgb(0.10, 0.12, 0.13)
            };

            commands.spawn((
                Sprite::from_color(color, Vec2::splat(CELL_SIZE - 2.0)),
                Transform::from_translation(grid_to_world(pos).extend(0.0)),
                GameWorld,
            ));
        }
    }
}

fn spawn_path_markers(commands: &mut Commands, path: &[GridPos], show_path_overlay: bool) {
    if path.is_empty() || !show_path_overlay {
        return;
    }

    // The route is now sparse, any-angle waypoints, so lay evenly spaced dots
    // along each straight run to keep a continuous trail.
    let spacing = CELL_SIZE * 1.05;
    let mut points: Vec<Vec2> = Vec::new();
    for window in path.windows(2) {
        let start = grid_to_world(window[0]);
        let end = grid_to_world(window[1]);
        let steps = ((start.distance(end) / spacing).round() as i32).max(1);
        for step in 0..steps {
            points.push(start.lerp(end, step as f32 / steps as f32));
        }
    }
    points.push(grid_to_world(*path.last().unwrap()));

    for point in points {
        commands.spawn((
            Sprite::from_color(
                Color::srgba(0.88, 0.78, 0.42, 0.10),
                Vec2::splat(CELL_SIZE * 0.22),
            ),
            Transform::from_translation(point.extend(1.0)),
            PathMarker,
            GameWorld,
        ));
    }
}

fn spawn_checkpoint_markers(commands: &mut Commands, checkpoints: &[GridPos]) {
    let mut labels_by_position: HashMap<GridPos, Vec<String>> = HashMap::new();
    for (index, checkpoint) in checkpoints.iter().enumerate() {
        labels_by_position
            .entry(*checkpoint)
            .or_default()
            .push((index + 1).to_string());
    }

    for (checkpoint, labels) in labels_by_position {
        commands.spawn((
            Sprite::from_color(Color::srgb(0.86, 0.68, 0.16), Vec2::splat(CELL_SIZE * 0.62)),
            Transform::from_translation(grid_to_world(checkpoint).extend(2.0)),
            CheckpointMarker,
            GameWorld,
        ));
        commands.spawn((
            Text2d::new(labels.join("/")),
            TextFont {
                font_size: if labels.len() > 1 { 13.0 } else { 26.0 },
                ..default()
            },
            TextColor(Color::srgb(0.08, 0.08, 0.07)),
            Transform::from_translation(grid_to_world(checkpoint).extend(3.0)),
            CheckpointMarker,
            GameWorld,
        ));
    }
}

fn spawn_escape_menu(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
            EscapeMenu,
            GameWorld,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(360),
                        padding: UiRect::all(px(20)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(12),
                        align_items: AlignItems::Stretch,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.04, 0.045, 0.052, 0.97)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Paused"),
                        TextFont {
                            font_size: 32.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.94, 0.96, 0.91)),
                    ));
                    spawn_escape_button(panel, "Reset", EscapeMenuAction::Reset);
                    spawn_escape_button(panel, "Main Menu", EscapeMenuAction::Home);
                    spawn_escape_button(panel, "How to Play", EscapeMenuAction::HowToPlay);
                    panel.spawn((
                        Text::new("Press Esc to resume"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.62, 0.68, 0.68)),
                    ));
                });
        });
}

fn spawn_escape_button(parent: &mut ChildSpawnerCommands, label: &str, action: EscapeMenuAction) {
    parent
        .spawn((
            Button,
            Node {
                height: px(46),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.18, 0.23, 0.24)),
            EscapeMenuButton { action },
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.97, 0.92)),
            ));
        });
}

fn spawn_escape_how_to_play(commands: &mut Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(86),
            right: px(24),
            width: px(390),
            padding: UiRect::all(px(18)),
            flex_direction: FlexDirection::Column,
            row_gap: px(10),
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.06, 0.065, 0.97)),
        EscapeMenu,
        EscapeMenuInfo,
        GameWorld,
        children![
            (
                Text::new("How to Play"),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.94, 0.96, 0.91)),
            ),
            (
                Text::new(how_to_play_text()),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.82, 0.80)),
            )
        ],
    ));
}

fn reset_camera(camera: &mut Query<(&mut Transform, &mut Projection), With<Camera2d>>) {
    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };

    transform.translation.x = 0.0;
    transform.translation.y = 0.0;

    if let Projection::Orthographic(projection) = &mut *projection {
        projection.scale = 1.0;
    }
}

fn spawn_offer_bar(commands: &mut Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(52),
                left: px(0),
                width: percent(100),
                height: px(96),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(12),
                ..default()
            },
            BackgroundColor(Color::srgba(0.035, 0.04, 0.045, 0.90)),
            GameWorld,
        ))
        .with_children(|bar| {
            for index in 0..OFFER_COUNT {
                bar.spawn((
                    Button,
                    Node {
                        width: px(96),
                        height: px(78),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(4),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.08, 0.09, 0.10, 0.94)),
                    OfferButton { index },
                ))
                .with_children(|offer| {
                    offer.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: px(0),
                            height: px(0),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                        OfferSelectionGlow { index },
                    ));
                    offer.spawn((
                        ImageNode::default(),
                        Node {
                            width: px(34),
                            height: px(34),
                            ..default()
                        },
                        OfferVisual { index },
                    ));
                    offer.spawn((
                        Text::new(""),
                        TextFont {
                            font_size: 10.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.93, 0.94, 0.95)),
                        OfferLabel { index },
                    ));
                });
            }
        });
}

fn spawn_home_screen(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.035, 0.04, 0.045), Vec2::new(1280.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        HomeScreen,
    ));

    spawn_crystal_cluster(commands, Vec2::new(-330.0, -120.0), 1.25, HomeScreen);
    spawn_crystal_cluster(commands, Vec2::new(352.0, 122.0), 0.9, HomeScreen);
    spawn_floating_gem(
        commands,
        Vec2::new(0.0, 120.0),
        112.0,
        Color::srgb(0.08, 0.72, 0.58),
        HomeScreen,
    );
    spawn_floating_gem(
        commands,
        Vec2::new(-118.0, 66.0),
        56.0,
        Color::srgb(0.92, 0.08, 0.12),
        HomeScreen,
    );
    spawn_floating_gem(
        commands,
        Vec2::new(122.0, 54.0),
        62.0,
        Color::srgb(0.12, 0.33, 0.95),
        HomeScreen,
    );

    commands.spawn((
        Text2d::new("Geode TD"),
        TextFont {
            font_size: 76.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, 262.0, 20.0),
        HomeScreen,
    ));
    commands.spawn((
        Text2d::new("Shape the route. Fuse the gems. Hold the line."),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.70, 0.78, 0.76)),
        Transform::from_xyz(0.0, 212.0, 20.0),
        HomeScreen,
    ));

    spawn_menu_button(
        commands,
        Vec2::new(0.0, -50.0),
        Vec2::new(250.0, 58.0),
        "Play",
        MenuAction::Play,
        HomeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -124.0),
        Vec2::new(250.0, 58.0),
        "How to Play",
        MenuAction::HowToPlay,
        HomeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -198.0),
        Vec2::new(250.0, 58.0),
        "Settings",
        MenuAction::Settings,
        HomeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -272.0),
        Vec2::new(250.0, 58.0),
        "Quit",
        MenuAction::Quit,
        HomeScreen,
    );
}

fn spawn_play_type_screen(commands: &mut Commands) {
    spawn_menu_backdrop(commands, PlayTypeScreen);
    spawn_title(commands, "Play", 188.0, PlayTypeScreen);
    spawn_subtitle(
        commands,
        "Choose whether this run is local or connected to a lobby.",
        128.0,
        PlayTypeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, 14.0),
        Vec2::new(270.0, 58.0),
        "Single Player",
        MenuAction::SinglePlayer,
        PlayTypeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -62.0),
        Vec2::new(270.0, 58.0),
        "Multiplayer",
        MenuAction::Multiplayer,
        PlayTypeScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -164.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        PlayTypeScreen,
    );
}

fn spawn_multiplayer_menu_screen(commands: &mut Commands) {
    spawn_menu_backdrop(commands, MultiplayerMenuScreen);
    spawn_title(commands, "Multiplayer", 188.0, MultiplayerMenuScreen);
    spawn_subtitle(
        commands,
        "Host a lobby or join one that is waiting to start.",
        128.0,
        MultiplayerMenuScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, 14.0),
        Vec2::new(250.0, 58.0),
        "Host",
        MenuAction::Host,
        MultiplayerMenuScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -62.0),
        Vec2::new(250.0, 58.0),
        "Join",
        MenuAction::Join,
        MultiplayerMenuScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -164.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        MultiplayerMenuScreen,
    );
}

fn spawn_display_name_screen(commands: &mut Commands, multiplayer: &MultiplayerClient) {
    spawn_menu_backdrop(commands, DisplayNameScreen);
    spawn_title(commands, "Display Name", 188.0, DisplayNameScreen);
    spawn_subtitle(
        commands,
        "This name is saved on this computer and used in lobbies.",
        128.0,
        DisplayNameScreen,
    );
    spawn_text_input(
        commands,
        Vec2::new(0.0, 18.0),
        Vec2::new(360.0, 54.0),
        &multiplayer.display_name_input,
        TextInputField::DisplayName,
        DisplayNameScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -74.0),
        Vec2::new(250.0, 54.0),
        "Continue",
        MenuAction::SubmitDisplayName,
        DisplayNameScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -164.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        DisplayNameScreen,
    );
}

fn spawn_host_lobby_screen(commands: &mut Commands, multiplayer: &MultiplayerClient) {
    spawn_menu_backdrop(commands, HostLobbyScreen);
    spawn_title(commands, "Host Lobby", 188.0, HostLobbyScreen);
    spawn_subtitle(
        commands,
        "Enter a lobby name, then create a waiting room.",
        128.0,
        HostLobbyScreen,
    );
    spawn_text_input(
        commands,
        Vec2::new(0.0, 18.0),
        Vec2::new(420.0, 54.0),
        &multiplayer.lobby_name_input,
        TextInputField::LobbyName,
        HostLobbyScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -74.0),
        Vec2::new(250.0, 54.0),
        "Create Lobby",
        MenuAction::CreateLobby,
        HostLobbyScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -164.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        HostLobbyScreen,
    );
}

fn spawn_join_lobby_screen(commands: &mut Commands, multiplayer: &MultiplayerClient) {
    spawn_menu_backdrop(commands, JoinLobbyScreen);
    spawn_title(commands, "Join Lobby", 250.0, JoinLobbyScreen);
    spawn_subtitle(
        commands,
        "Waiting lobbies refresh automatically.",
        196.0,
        JoinLobbyScreen,
    );
    commands.spawn((
        Text2d::new(multiplayer.status.clone()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.66, 0.74, 0.74)),
        Transform::from_xyz(0.0, 162.0, 20.0),
        MultiplayerStatusText,
        JoinLobbyScreen,
    ));
    spawn_lobby_rows(commands, multiplayer, JoinLobbyScreen);
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -264.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        JoinLobbyScreen,
    );
}

fn spawn_waiting_lobby_screen(commands: &mut Commands, multiplayer: &MultiplayerClient) {
    spawn_menu_backdrop(commands, WaitingLobbyScreen);
    spawn_title(commands, "Lobby", 250.0, WaitingLobbyScreen);
    commands.spawn((
        Text2d::new(multiplayer.status.clone()),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.66, 0.74, 0.74)),
        Transform::from_xyz(0.0, 196.0, 20.0),
        MultiplayerStatusText,
        WaitingLobbyScreen,
    ));
    spawn_waiting_lobby_dynamic(commands, multiplayer);
    spawn_menu_button(
        commands,
        Vec2::new(-124.0, -264.0),
        Vec2::new(220.0, 52.0),
        "Leave",
        MenuAction::LeaveLobby,
        WaitingLobbyScreen,
    );
}

fn spawn_mode_select_screen(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.035, 0.04, 0.045), Vec2::new(1280.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        ModeSelectScreen,
    ));
    spawn_crystal_cluster(commands, Vec2::new(-330.0, -120.0), 1.0, ModeSelectScreen);
    spawn_crystal_cluster(commands, Vec2::new(340.0, 100.0), 0.85, ModeSelectScreen);
    commands.spawn((
        Text2d::new("Choose Mode"),
        TextFont {
            font_size: 58.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, 188.0, 20.0),
        ModeSelectScreen,
    ));
    commands.spawn((
        Text2d::new("Standard uses fixed route points. Random rolls a fresh layout."),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.70, 0.78, 0.76)),
        Transform::from_xyz(0.0, 128.0, 20.0),
        ModeSelectScreen,
    ));
    spawn_menu_button(
        commands,
        Vec2::new(0.0, 10.0),
        Vec2::new(250.0, 58.0),
        "Standard",
        MenuAction::Standard,
        ModeSelectScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -64.0),
        Vec2::new(250.0, 58.0),
        "Random",
        MenuAction::Random,
        ModeSelectScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -164.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        ModeSelectScreen,
    );
}

fn spawn_how_to_play_screen(commands: &mut Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.035, 0.04, 0.045), Vec2::new(1280.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        HowToPlayScreen,
    ));
    spawn_floating_gem(
        commands,
        Vec2::new(-388.0, 160.0),
        78.0,
        Color::srgb(0.08, 0.72, 0.34),
        HowToPlayScreen,
    );
    spawn_floating_gem(
        commands,
        Vec2::new(392.0, -142.0),
        88.0,
        Color::srgb(0.12, 0.33, 0.95),
        HowToPlayScreen,
    );
    commands.spawn((
        Text2d::new("How to Play"),
        TextFont {
            font_size: 58.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, 218.0, 20.0),
        HowToPlayScreen,
    ));
    commands.spawn((
        Text2d::new(how_to_play_text()),
        TextFont {
            font_size: 19.0,
            ..default()
        },
        TextColor(Color::srgb(0.76, 0.82, 0.80)),
        Transform::from_xyz(0.0, 56.0, 20.0),
        HowToPlayScreen,
    ));
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -196.0),
        Vec2::new(220.0, 52.0),
        "Back",
        MenuAction::Home,
        HowToPlayScreen,
    );
}

fn how_to_play_text() -> &'static str {
    "Place all five chipped gems each build round, then click one to keep.\n\
The four unpicked starters become stone walls that bend enemy pathing.\n\
Enemies must travel through each numbered point before reaching the end.\n\
Towers cannot block the route, but they can bend it.\n\
Click a tower to upgrade it by sacrificing a matching duplicate.\n\
Each enemy killed grants one coin.\n\
Watch the left panel: swift waves rush in, flying waves skip the maze,\n\
and every 20th round sends a single high-health boss.\n\
Use the mouse wheel to zoom, and hold left click to pan."
}

fn spawn_settings_screen(commands: &mut Commands, show_path_overlay: bool) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.04, 0.045, 0.052), Vec2::new(1280.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        SettingsScreen,
    ));
    spawn_floating_gem(
        commands,
        Vec2::new(-360.0, 180.0),
        76.0,
        Color::srgb(0.68, 0.22, 0.92),
        SettingsScreen,
    );
    spawn_floating_gem(
        commands,
        Vec2::new(360.0, -160.0),
        96.0,
        Color::srgb(1.0, 0.74, 0.12),
        SettingsScreen,
    );
    commands.spawn((
        Text2d::new("Settings"),
        TextFont {
            font_size: 58.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, 158.0, 20.0),
        SettingsScreen,
    ));
    commands.spawn((
        Text2d::new("Visual options"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::srgb(0.66, 0.72, 0.72)),
        Transform::from_xyz(0.0, 92.0, 20.0),
        SettingsScreen,
    ));
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -54.0),
        Vec2::new(300.0, 58.0),
        if show_path_overlay {
            "Path Overlay: On"
        } else {
            "Path Overlay: Off"
        },
        MenuAction::TogglePathOverlay,
        SettingsScreen,
    );
    spawn_menu_button(
        commands,
        Vec2::new(0.0, -146.0),
        Vec2::new(220.0, 58.0),
        "Back",
        MenuAction::Home,
        SettingsScreen,
    );
}

fn spawn_menu_backdrop<T: Component + Clone>(commands: &mut Commands, marker: T) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.035, 0.04, 0.045), Vec2::new(1280.0, 760.0)),
        Transform::from_xyz(0.0, 0.0, -10.0),
        marker.clone(),
    ));
    spawn_crystal_cluster(commands, Vec2::new(-330.0, -120.0), 1.0, marker.clone());
    spawn_crystal_cluster(commands, Vec2::new(340.0, 100.0), 0.85, marker);
}

fn spawn_title<T: Component>(commands: &mut Commands, title: &str, y: f32, marker: T) {
    commands.spawn((
        Text2d::new(title),
        TextFont {
            font_size: 58.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, y, 20.0),
        marker,
    ));
}

fn spawn_subtitle<T: Component>(commands: &mut Commands, text: &str, y: f32, marker: T) {
    commands.spawn((
        Text2d::new(text),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.70, 0.78, 0.76)),
        Transform::from_xyz(0.0, y, 20.0),
        marker,
    ));
}

fn spawn_text_input<T: Component + Clone>(
    commands: &mut Commands,
    center: Vec2,
    size: Vec2,
    value: &str,
    field: TextInputField,
    marker: T,
) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.08, 0.10, 0.11), size),
        Transform::from_translation(center.extend(12.0)),
        marker.clone(),
    ));
    commands.spawn((
        Text2d::new(text_input_value(value)),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.92)),
        Transform::from_xyz(center.x, center.y - 6.0, 20.0),
        TextInputDisplay { field },
        marker,
    ));
}

fn spawn_lobby_rows<T: Component + Clone>(
    commands: &mut Commands,
    multiplayer: &MultiplayerClient,
    marker: T,
) {
    let waiting = multiplayer
        .lobbies
        .iter()
        .filter(|lobby| lobby.status == geode_td_shared::LobbyStatus::Waiting)
        .collect::<Vec<_>>();

    if waiting.is_empty() {
        commands.spawn((
            Text2d::new("No waiting lobbies found."),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.74, 0.80, 0.80)),
            Transform::from_xyz(0.0, 44.0, 20.0),
            MultiplayerDynamic,
            marker,
        ));
        return;
    }

    for (index, lobby) in waiting.into_iter().take(6).enumerate() {
        let center = Vec2::new(0.0, 94.0 - index as f32 * 62.0);
        let size = Vec2::new(560.0, 48.0);
        let label = format!(
            "{}    Host: {}    {}/{}",
            lobby.name, lobby.host_name, lobby.player_count, lobby.max_players
        );
        commands.spawn((
            Sprite::from_color(Color::srgb(0.13, 0.17, 0.18), size),
            Transform::from_translation(center.extend(12.0)),
            LobbyRowButton {
                lobby_id: lobby.id.clone(),
                center,
                size,
            },
            MultiplayerDynamic,
            marker.clone(),
        ));
        commands.spawn((
            Text2d::new(label),
            TextFont {
                font_size: 19.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.95, 0.93)),
            Transform::from_xyz(center.x, center.y - 5.0, 20.0),
            MultiplayerDynamic,
            marker.clone(),
        ));
    }
}

fn spawn_waiting_lobby_dynamic(commands: &mut Commands, multiplayer: &MultiplayerClient) {
    let Some(lobby) = &multiplayer.lobby else {
        commands.spawn((
            Text2d::new("Waiting for server lobby state..."),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.74, 0.80, 0.80)),
            Transform::from_xyz(0.0, 72.0, 20.0),
            MultiplayerDynamic,
            WaitingLobbyScreen,
        ));
        return;
    };

    let header = format!(
        "{}    {}/{}",
        lobby.name,
        lobby.players.len(),
        lobby.max_players
    );
    commands.spawn((
        Text2d::new(header),
        TextFont {
            font_size: 28.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.96, 0.91)),
        Transform::from_xyz(0.0, 142.0, 20.0),
        MultiplayerDynamic,
        WaitingLobbyScreen,
    ));

    for (index, player) in lobby.players.iter().take(8).enumerate() {
        let role = if player.is_host { "Host" } else { "Player" };
        commands.spawn((
            Text2d::new(format!("{}    {}", player.name, role)),
            TextFont {
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(0.82, 0.88, 0.86)),
            Transform::from_xyz(0.0, 92.0 - index as f32 * 34.0, 20.0),
            MultiplayerDynamic,
            WaitingLobbyScreen,
        ));
    }

    let is_host = multiplayer
        .player_id
        .as_ref()
        .is_some_and(|id| id == &lobby.host_player_id);
    if is_host {
        spawn_dynamic_menu_button(
            commands,
            Vec2::new(124.0, -264.0),
            Vec2::new(220.0, 52.0),
            "Start Game",
            MenuAction::StartMultiplayerGame,
            WaitingLobbyScreen,
        );
    }
}

fn text_input_value(value: &str) -> String {
    if value.is_empty() {
        "|".to_string()
    } else {
        format!("{value}|")
    }
}

fn max_input_chars(field: TextInputField) -> usize {
    match field {
        TextInputField::DisplayName => 32,
        TextInputField::LobbyName => 40,
    }
}

fn spawn_menu_button<T: Component + Clone>(
    commands: &mut Commands,
    center: Vec2,
    size: Vec2,
    label: &str,
    action: MenuAction,
    marker: T,
) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.18, 0.23, 0.24), size),
        Transform::from_translation(center.extend(12.0)),
        MenuButton {
            action,
            center,
            size,
        },
        marker.clone(),
    ));
    commands.spawn((
        Text2d::new(label),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.97, 0.92)),
        Transform::from_xyz(center.x, center.y - 5.0, 20.0),
        marker,
    ));
}

fn spawn_dynamic_menu_button<T: Component + Clone>(
    commands: &mut Commands,
    center: Vec2,
    size: Vec2,
    label: &str,
    action: MenuAction,
    marker: T,
) {
    commands.spawn((
        Sprite::from_color(Color::srgb(0.18, 0.23, 0.24), size),
        Transform::from_translation(center.extend(12.0)),
        MenuButton {
            action,
            center,
            size,
        },
        MultiplayerDynamic,
        marker.clone(),
    ));
    commands.spawn((
        Text2d::new(label),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.97, 0.92)),
        Transform::from_xyz(center.x, center.y - 5.0, 20.0),
        MultiplayerDynamic,
        marker,
    ));
}

fn spawn_floating_gem<T: Component>(
    commands: &mut Commands,
    center: Vec2,
    size: f32,
    color: Color,
    marker: T,
) {
    commands.spawn((
        Sprite::from_color(color, Vec2::splat(size)),
        Transform::from_translation(center.extend(2.0))
            .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
        marker,
    ));
}

fn spawn_crystal_cluster<T: Component + Clone>(
    commands: &mut Commands,
    center: Vec2,
    scale: f32,
    marker: T,
) {
    let gems = [
        (Vec2::new(-44.0, -4.0), 62.0, Color::srgb(0.12, 0.33, 0.95)),
        (Vec2::new(14.0, 26.0), 88.0, Color::srgb(0.08, 0.72, 0.34)),
        (Vec2::new(72.0, -18.0), 58.0, Color::srgb(0.86, 0.96, 1.0)),
    ];

    for (offset, size, color) in gems {
        spawn_floating_gem(
            commands,
            center + offset * scale,
            size * scale,
            color,
            marker.clone(),
        );
    }
}

fn despawn_all<T: Component>(commands: &mut Commands, query: &Query<Entity, With<T>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn despawn_entities<F: QueryFilter>(commands: &mut Commands, query: &Query<Entity, F>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn point_in_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    point.x >= center.x - size.x * 0.5
        && point.x <= center.x + size.x * 0.5
        && point.y >= center.y - size.y * 0.5
        && point.y <= center.y + size.y * 0.5
}

fn cursor_world_position(
    windows: &Query<&Window, With<PrimaryWindow>>,
    camera: &Query<(&Camera, &GlobalTransform)>,
) -> Option<Vec2> {
    let window = windows.single().ok()?;
    let (camera, camera_transform) = camera.single().ok()?;
    let cursor_position = window.cursor_position()?;
    camera
        .viewport_to_world_2d(camera_transform, cursor_position)
        .ok()
}

fn grade_ladder_text() -> String {
    GRADE_LADDER
        .iter()
        .map(|grade| grade.name())
        .collect::<Vec<_>>()
        .join(" > ")
}
