use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::HashMap;

use crate::board::Board;
use crate::components::{
    CheckpointMarker, EscapeMenu, GameWorld, HomeScreen, HowToPlayScreen, HudText, MenuAction,
    MenuButton, ModeSelectScreen, OfferLabel, OfferVisual, PathMarker, SelectionMenu,
    SettingsScreen, Tower, UpgradeButton,
};
use crate::constants::{CELL_SIZE, OFFER_COUNT};
use crate::game::{AppScreen, Game, GameMode, Phase};
use crate::gem::{GRADE_LADDER, GemGrade};
use crate::grid::{GridPos, finish_pos, grid_to_world, offer_x, start_pos};

const OFFER_GEM_Y: f32 = -302.0;
const OFFER_LABEL_Y: f32 = -348.0;
const OFFER_HIT_WIDTH: f32 = 98.0;
const OFFER_HIT_HEIGHT: f32 = 96.0;

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
            game.screen = AppScreen::HowToPlay;
        }
        MenuAction::Settings => {
            despawn_all(&mut commands, &home_entities);
            spawn_settings_screen(&mut commands);
            game.screen = AppScreen::Settings;
        }
        MenuAction::Home => {
            despawn_all(&mut commands, &home_entities);
            despawn_all(&mut commands, &mode_entities);
            despawn_all(&mut commands, &how_to_play_entities);
            despawn_all(&mut commands, &settings_entities);
            despawn_all(&mut commands, &game_entities);
            spawn_home_screen(&mut commands);
            game.screen = AppScreen::Home;
        }
    }
}

pub fn offer_index_at(world_pos: Vec2) -> Option<usize> {
    (0..OFFER_COUNT).find(|index| {
        let center = Vec2::new(offer_x(*index), (OFFER_GEM_Y + OFFER_LABEL_Y) * 0.5);
        point_in_rect(
            world_pos,
            center,
            Vec2::new(OFFER_HIT_WIDTH, OFFER_HIT_HEIGHT),
        )
    })
}

fn spawn_game_scene(commands: &mut Commands, board: &Board) {
    spawn_board_tiles(commands, board);
    spawn_path_markers(commands, &board.path);
    spawn_checkpoint_markers(commands, &board.checkpoints);
    spawn_offer_bar(commands);
}

pub fn toggle_escape_menu(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    game: Res<Game>,
    menu_items: Query<Entity, With<EscapeMenu>>,
) {
    if game.screen != AppScreen::Playing || !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    if menu_items.iter().next().is_some() {
        despawn_all(&mut commands, &menu_items);
    } else {
        spawn_escape_menu(&mut commands);
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
    let center = selection_menu_center();
    let x = center.x;
    let y = center.y;
    let title = format!("{} {}", tower.grade.name(), tower.gem.name());
    let action = match tower.grade.next() {
        Some(next) => format!("Upgrade to {}", next.name()),
        None => "Perfect grade".to_string(),
    };

    commands.spawn((
        Sprite::from_color(Color::srgb(0.12, 0.13, 0.14), Vec2::new(236.0, 150.0)),
        Transform::from_xyz(x, y, 120.0),
        SelectionMenu,
        GameWorld,
    ));

    commands.spawn((
        Text2d::new(title),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.95, 0.95)),
        Transform::from_xyz(x, y + 46.0, 130.0),
        SelectionMenu,
        GameWorld,
    ));

    commands.spawn((
        Sprite::from_color(Color::srgb(0.20, 0.25, 0.28), Vec2::new(176.0, 46.0)),
        Transform::from_xyz(upgrade_button_center().x, upgrade_button_center().y, 130.0),
        SelectionMenu,
        UpgradeButton,
        GameWorld,
    ));

    commands.spawn((
        Text2d::new(action),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.96, 0.92)),
        Transform::from_xyz(
            upgrade_button_center().x,
            upgrade_button_center().y - 4.0,
            140.0,
        ),
        SelectionMenu,
        GameWorld,
    ));
}

pub fn update_offer_visuals(
    game: Res<Game>,
    mut offer_sprites: Query<(&OfferVisual, &mut Sprite, &mut Transform)>,
    mut offer_labels: Query<(&OfferLabel, &mut Text2d, &mut TextColor)>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for (offer, mut sprite, mut transform) in &mut offer_sprites {
        let selected = game.selected_offer == offer.index && game.phase == Phase::Build;
        let size = if selected { 44.0 } else { 34.0 };
        sprite.custom_size = Some(Vec2::splat(size));
        transform.scale = if selected {
            Vec3::splat(1.05)
        } else {
            Vec3::ONE
        };

        sprite.color = match game.offers[offer.index] {
            Some(gem) => gem.color(),
            None => Color::srgb(0.08, 0.085, 0.09),
        };
    }

    for (label, mut text, mut color) in &mut offer_labels {
        text.0 = match game.offers[label.index] {
            Some(gem) => format!("Chipped\n{}", gem.name()),
            None => "--".to_string(),
        };
        color.0 = if game.offers[label.index].is_some() {
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
    let prompt = if game.upgrade_source.is_some() {
        "      Upgrade: click a matching duplicate to sacrifice"
    } else {
        ""
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

pub fn upgrade_button_center() -> Vec2 {
    selection_menu_center() + Vec2::new(0.0, -8.0)
}

pub fn is_upgrade_button_click(world_pos: Vec2) -> bool {
    let center = upgrade_button_center();
    world_pos.x >= center.x - 88.0
        && world_pos.x <= center.x + 88.0
        && world_pos.y >= center.y - 23.0
        && world_pos.y <= center.y + 23.0
}

pub fn tower_sprite_size(grade: GemGrade) -> Vec2 {
    Vec2::splat(CELL_SIZE * 0.58 * grade.size_multiplier())
}

fn selection_menu_center() -> Vec2 {
    Vec2::new(450.0, 82.0)
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
    commands.spawn((
        Sprite::from_color(
            Color::srgba(0.02, 0.025, 0.03, 0.90),
            Vec2::new(760.0, 290.0),
        ),
        Transform::from_xyz(0.0, 34.0, 180.0),
        EscapeMenu,
        GameWorld,
    ));
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.94, 0.94)),
        Transform::from_xyz(0.0, 76.0, 190.0),
        HudText,
        EscapeMenu,
        GameWorld,
    ));
    commands.spawn((
        Text2d::new("Esc"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.62, 0.68, 0.68)),
        Transform::from_xyz(0.0, -88.0, 190.0),
        EscapeMenu,
        GameWorld,
    ));
}

fn spawn_offer_bar(commands: &mut Commands) {
    for index in 0..OFFER_COUNT {
        let x = offer_x(index);

        commands.spawn((
            Sprite::from_color(Color::srgb(0.16, 0.17, 0.18), Vec2::splat(48.0)),
            Transform::from_xyz(x, OFFER_GEM_Y, 90.0)
                .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            OfferVisual { index },
            GameWorld,
        ));

        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgb(0.93, 0.94, 0.95)),
            Transform::from_xyz(x, OFFER_LABEL_Y, 100.0),
            OfferLabel { index },
            GameWorld,
        ));
    }
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
        Text2d::new(
            "Pick one chipped gem each round and place it on the board.\n\
Enemies must travel through each numbered point before reaching the end.\n\
Towers cannot block the route, but they can bend it.\n\
Click a tower to upgrade it by sacrificing a matching duplicate.\n\
Each enemy killed grants one coin.",
        ),
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
