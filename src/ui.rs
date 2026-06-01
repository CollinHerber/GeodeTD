use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

use crate::board::Board;
use crate::components::{
    CheckpointMarker, EscapeMenu, EscapeMenuAction, EscapeMenuButton, EscapeMenuInfo, GameWorld,
    HomeScreen, HowToPlayScreen, HudText, MenuAction, MenuButton, ModeSelectScreen, OfferButton,
    OfferLabel, OfferVisual, PathMarker, SelectionMenu, SettingsScreen, SpeedButton, SpeedText,
    TopBarText, Tower, UpgradeButton,
};
use crate::constants::{CELL_SIZE, OFFER_COUNT};
use crate::game::{AppScreen, Game, GameMode, Phase};
use crate::gem::{GRADE_LADDER, GemEffect, GemGrade, GemKind};
use crate::gem_visual::GemImages;
use crate::grid::{GridPos, finish_pos, grid_to_world, start_pos};

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
    buttons_query: Query<&MenuButton>,
    home_entities: Query<Entity, With<HomeScreen>>,
    mode_entities: Query<Entity, With<ModeSelectScreen>>,
    how_to_play_entities: Query<Entity, With<HowToPlayScreen>>,
    settings_entities: Query<Entity, With<SettingsScreen>>,
    game_entities: Query<Entity, With<GameWorld>>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(world_pos) = cursor_world_position(&windows, &camera) else {
        return;
    };

    let Some(action) = buttons_query
        .iter()
        .find(|button| point_in_rect(world_pos, button.center, button.size))
        .map(|button| button.action)
    else {
        return;
    };

    match action {
        MenuAction::Play => {
            despawn_all(&mut commands, &home_entities);
            spawn_mode_select_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::ModeSelect;
        }
        MenuAction::Standard => {
            despawn_all(&mut commands, &home_entities);
            despawn_all(&mut commands, &mode_entities);
            despawn_all(&mut commands, &how_to_play_entities);
            despawn_all(&mut commands, &settings_entities);
            despawn_all(&mut commands, &game_entities);
            board.reset_for_mode(GameMode::Standard);
            game.reset_for_mode(GameMode::Standard);
            spawn_game_scene(&mut commands, &board);
            game.screen = AppScreen::Playing;
        }
        MenuAction::Random => {
            despawn_all(&mut commands, &home_entities);
            despawn_all(&mut commands, &mode_entities);
            despawn_all(&mut commands, &how_to_play_entities);
            despawn_all(&mut commands, &settings_entities);
            despawn_all(&mut commands, &game_entities);
            board.reset_for_mode(GameMode::Random);
            game.reset_for_mode(GameMode::Random);
            spawn_game_scene(&mut commands, &board);
            game.screen = AppScreen::Playing;
        }
        MenuAction::HowToPlay => {
            despawn_all(&mut commands, &home_entities);
            spawn_how_to_play_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::HowToPlay;
        }
        MenuAction::Settings => {
            despawn_all(&mut commands, &home_entities);
            spawn_settings_screen(&mut commands);
            game.paused = false;
            game.screen = AppScreen::Settings;
        }
        MenuAction::Home => {
            despawn_all(&mut commands, &home_entities);
            despawn_all(&mut commands, &mode_entities);
            despawn_all(&mut commands, &how_to_play_entities);
            despawn_all(&mut commands, &settings_entities);
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
                spawn_game_scene(&mut commands, &board);
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

fn spawn_game_scene(commands: &mut Commands, board: &Board) {
    spawn_board_tiles(commands, board);
    spawn_path_markers(commands, &board.path);
    spawn_checkpoint_markers(commands, &board.checkpoints);
    spawn_play_ui(commands);
    spawn_offer_bar(commands);
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
) {
    for entity in path_markers.iter() {
        commands.entity(entity).despawn();
    }
    spawn_path_markers(commands, path);
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
    let title = format!("{} {}", tower.grade.name(), tower.gem.name());
    let action = match tower.grade.next() {
        Some(next) => format!("Upgrade to {}", next.name()),
        None => "Perfect grade".to_string(),
    };

    let damage = tower.damage * tower.grade.damage_multiplier();
    let stats = stats_text(
        damage,
        tower.range,
        tower.cooldown.duration().as_secs_f32(),
        tower.gem.effect(tower.grade),
    );

    spawn_info_panel(commands, &title, &stats);
    spawn_action_bar(commands, &action);
}

/// Read-only stat panel for a gem offer (before placement). Shows the same stats
/// as a placed tower at Chipped grade, but without the upgrade button.
pub fn spawn_gem_info(commands: &mut Commands, gem: GemKind) {
    let stats = gem.chipped_stats();
    let body = stats_text(
        stats.damage,
        stats.range,
        stats.cooldown,
        gem.effect(GemGrade::Chipped),
    );

    spawn_info_panel(commands, &format!("Chipped {}", gem.name()), &body);
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

fn spawn_action_bar(commands: &mut Commands, action: &str) {
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
                ));
            });
        });
}

fn stats_text(damage: f32, range: f32, cooldown: f32, effect: GemEffect) -> String {
    let fire_rate = if cooldown > 0.0 { 1.0 / cooldown } else { 0.0 };
    format!(
        "Damage: {:.0}\nRange: {:.0}\nFire rate: {:.2}/s\n{}",
        damage,
        range,
        fire_rate,
        effect.describe()
    )
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
        "Wave {}        Lives {}        Coins {}",
        game.round,
        game.lives.max(0),
        game.coins
    );

    if let Ok(mut speed) = speed.single_mut() {
        **speed = format!("{}x", game.speed);
    }
}

pub fn update_offer_visuals(
    game: Res<Game>,
    gem_images: Res<GemImages>,
    mut offer_buttons: Query<(&OfferButton, &mut BackgroundColor)>,
    mut offer_sprites: Query<(&OfferVisual, &mut ImageNode, &mut Node)>,
    mut offer_labels: Query<(&OfferLabel, &mut Text, &mut TextColor)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (offer, mut color) in &mut offer_buttons {
        let selected = game.selected_offer == Some(offer.index) && game.phase == Phase::Build;
        color.0 = if selected {
            Color::srgb(0.20, 0.28, 0.28)
        } else if game.placed_starters[offer.index].is_some() {
            Color::srgba(0.11, 0.13, 0.13, 0.94)
        } else {
            Color::srgba(0.08, 0.09, 0.10, 0.94)
        };
    }

    for (offer, mut image, mut node) in &mut offer_sprites {
        let selected = game.selected_offer == Some(offer.index) && game.phase == Phase::Build;
        let size = if selected { 44.0 } else { 34.0 };
        node.width = px(size);
        node.height = px(size);

        image.image = match game.offers[offer.index] {
            Some(gem) => gem_images.handle(gem, GemGrade::Chipped),
            None => gem_images.empty(),
        };
        image.color = Color::WHITE;
    }

    for (label, mut text, mut color) in &mut offer_labels {
        **text = match (game.offers[label.index], game.placed_starters[label.index]) {
            (Some(gem), _) => format!("Chipped\n{}", gem.name()),
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
    let starter_prompt = if game.phase == Phase::Build && game.all_starters_placed() {
        "      Click a placed starter to keep it"
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
        "Mode: {}      Round: {}      Phase: {}{}\nLives: {}      Coins: {}      Path: {}\n{}{}\nGrades: {}",
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

pub fn tower_sprite_size(grade: GemGrade) -> Vec2 {
    Vec2::splat(CELL_SIZE * 0.58 * grade.size_multiplier())
}

fn spawn_board_tiles(commands: &mut Commands, board: &Board) {
    let protected = board.protected_cells();

    for row in 0..crate::constants::GRID_ROWS {
        for col in 0..crate::constants::GRID_COLUMNS {
            let pos = GridPos::new(col, row);
            let color = if pos == start_pos() {
                Color::srgb(0.10, 0.38, 0.18)
            } else if pos == finish_pos() {
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

fn spawn_path_markers(commands: &mut Commands, path: &[GridPos]) {
    for pos in path {
        commands.spawn((
            Sprite::from_color(
                Color::srgba(0.88, 0.78, 0.42, 0.28),
                Vec2::splat(CELL_SIZE * 0.38),
            ),
            Transform::from_translation(grid_to_world(*pos).extend(1.0)),
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
Use the mouse wheel to zoom, and hold left click to pan."
}

fn spawn_settings_screen(commands: &mut Commands) {
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
        Text2d::new("Settings will live here."),
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
        Vec2::new(0.0, -146.0),
        Vec2::new(220.0, 58.0),
        "Back",
        MenuAction::Home,
        SettingsScreen,
    );
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
