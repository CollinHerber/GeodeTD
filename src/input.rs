use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::time::Duration;

use crate::board::{Board, find_complete_path};
use crate::components::{GameWorld, PathMarker, SelectionMenu, Tower};
use crate::game::{AppScreen, Game, Phase};
use crate::gem::GemGrade;
use crate::gem_visual::GemImages;
use crate::grid::{grid_to_world, world_to_grid};
use crate::ui::{
    clear_selection_menu, offer_index_at, refresh_path_markers, spawn_gem_info,
    spawn_selection_menu, tower_sprite_size,
};

const MIN_CAMERA_SCALE: f32 = 0.45;
const MAX_CAMERA_SCALE: f32 = 2.8;
const ZOOM_STEP: f32 = 0.88;
const PIXELS_PER_SCROLL_LINE: f32 = 16.0;
const PAN_DRAG_THRESHOLD: f32 = 4.0;

#[derive(Resource, Default)]
pub struct CameraDrag {
    active: bool,
    distance: f32,
    suppress_click: bool,
}

pub fn pan_and_zoom_camera(
    game: Res<Game>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    motion: Res<AccumulatedMouseMotion>,
    mut drag: ResMut<CameraDrag>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
) {
    if game.screen != AppScreen::Playing {
        *drag = CameraDrag::default();
        return;
    }

    let Ok((mut transform, mut projection)) = camera.single_mut() else {
        return;
    };
    let Projection::Orthographic(projection) = &mut *projection else {
        return;
    };

    if scroll.delta.y != 0.0 {
        let scroll_lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / PIXELS_PER_SCROLL_LINE,
        };
        projection.scale = (projection.scale * ZOOM_STEP.powf(scroll_lines))
            .clamp(MIN_CAMERA_SCALE, MAX_CAMERA_SCALE);
    }

    if buttons.just_pressed(MouseButton::Left) {
        drag.active = true;
        drag.distance = 0.0;
        drag.suppress_click = false;
    }

    if drag.active && buttons.pressed(MouseButton::Left) && motion.delta != Vec2::ZERO {
        drag.distance += motion.delta.length();
        if drag.distance >= PAN_DRAG_THRESHOLD {
            transform.translation.x -= motion.delta.x * projection.scale;
            transform.translation.y += motion.delta.y * projection.scale;
        }
    }

    if buttons.just_released(MouseButton::Left) {
        drag.suppress_click = drag.suppress_click || drag.distance >= PAN_DRAG_THRESHOLD;
        drag.active = false;
        drag.distance = 0.0;
    }
}

pub fn select_offer(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mut game: ResMut<Game>,
    menu_items: Query<Entity, With<SelectionMenu>>,
) {
    if game.screen != AppScreen::Playing || game.phase != Phase::Build {
        return;
    }

    let requested = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
    ]
    .into_iter()
    .enumerate()
    .find_map(|(index, key)| keys.just_pressed(key).then_some(index));

    if let Some(index) = requested
        && let Some(gem) = game.offers[index]
    {
        game.selected_offer = index;
        game.selected_tower = None;
        game.upgrade_source = None;
        clear_selection_menu(&mut commands, &menu_items);
        spawn_gem_info(&mut commands, gem);
    }
}

pub fn handle_tower_action_clicks(
    mut commands: Commands,
    interactions: Query<
        &Interaction,
        (Changed<Interaction>, With<crate::components::UpgradeButton>),
    >,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    towers: Query<&Tower>,
    menu_items: Query<Entity, With<SelectionMenu>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            camera_drag.suppress_click = true;
            begin_upgrade_selection(&mut commands, &mut game, &towers, &menu_items);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn place_or_select(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    path_markers: Query<Entity, With<PathMarker>>,
    menu_items: Query<Entity, With<SelectionMenu>>,
    mut towers: Query<(Entity, &mut Tower, &mut Sprite)>,
    tower_positions: Query<(Entity, &Transform), With<Tower>>,
    gem_images: Res<GemImages>,
) {
    if game.screen != AppScreen::Playing || !buttons.just_released(MouseButton::Left) {
        return;
    }

    if camera_drag.suppress_click {
        camera_drag.suppress_click = false;
        return;
    }

    let Some(world_pos) = cursor_world_position(&windows, &camera) else {
        return;
    };

    if game.phase == Phase::Build
        && let Some(offer_index) = offer_index_at(world_pos)
    {
        if let Some(gem) = game.offers[offer_index] {
            game.selected_offer = offer_index;
            game.selected_tower = None;
            game.upgrade_source = None;
            clear_selection_menu(&mut commands, &menu_items);
            spawn_gem_info(&mut commands, gem);
        }
        return;
    }

    if let Some(tower_entity) = tower_at_world_position(world_pos, &tower_positions) {
        if game.phase == Phase::Build && game.upgrade_source.is_some() {
            complete_upgrade(
                &mut commands,
                &mut game,
                &mut board,
                &path_markers,
                &menu_items,
                &mut towers,
                &gem_images,
                tower_entity,
            );
        } else {
            select_tower(&mut commands, &mut game, &menu_items, &towers, tower_entity);
        }
        return;
    }

    if game.phase != Phase::Build {
        return;
    }

    let Some(grid_pos) = world_to_grid(world_pos) else {
        return;
    };

    if game.upgrade_source.is_some() {
        game.message = "Select a matching duplicate tower to sacrifice.".to_string();
        return;
    }

    place_tower(
        &mut commands,
        &mut game,
        &mut board,
        &path_markers,
        &menu_items,
        &gem_images,
        grid_pos,
    );
}

fn begin_upgrade_selection(
    commands: &mut Commands,
    game: &mut Game,
    towers: &Query<&Tower>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
) {
    let Some(source) = game.selected_tower else {
        return;
    };
    let Ok(tower) = towers.get(source) else {
        game.selected_tower = None;
        return;
    };

    if game.phase != Phase::Build {
        game.message = "Upgrades are available during build rounds.".to_string();
        return;
    }

    if tower.grade.next().is_none() {
        game.message = "That tower is already Perfect.".to_string();
        return;
    }

    game.upgrade_source = Some(source);
    game.message = format!(
        "Click another {} {} to sacrifice.",
        tower.grade.name(),
        tower.gem.name()
    );
    clear_selection_menu(commands, menu_items);
}

fn select_tower(
    commands: &mut Commands,
    game: &mut Game,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &Query<(Entity, &mut Tower, &mut Sprite)>,
    tower_entity: Entity,
) {
    let Ok((_, tower, _)) = towers.get(tower_entity) else {
        return;
    };

    game.selected_tower = Some(tower_entity);
    game.upgrade_source = None;
    game.message = format!("Selected {} {}.", tower.grade.name(), tower.gem.name());
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, tower);
}

#[allow(clippy::too_many_arguments)]
fn complete_upgrade(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    path_markers: &Query<Entity, With<PathMarker>>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &mut Query<(Entity, &mut Tower, &mut Sprite)>,
    gem_images: &GemImages,
    sacrifice_entity: Entity,
) {
    let Some(source_entity) = game.upgrade_source else {
        return;
    };

    if sacrifice_entity == source_entity {
        game.message = "Pick a different matching tower to sacrifice.".to_string();
        return;
    }

    let Ok(
        [
            (_, mut source_tower, mut source_sprite),
            (_, sacrifice_tower, _),
        ],
    ) = towers.get_many_mut([source_entity, sacrifice_entity])
    else {
        game.message = "That upgrade target is no longer available.".to_string();
        game.upgrade_source = None;
        return;
    };

    if source_tower.gem != sacrifice_tower.gem || source_tower.grade != sacrifice_tower.grade {
        game.message = format!(
            "Upgrade needs another {} {}.",
            source_tower.grade.name(),
            source_tower.gem.name()
        );
        return;
    }

    let Some(next_grade) = source_tower.grade.next() else {
        game.message = "That tower is already Perfect.".to_string();
        game.upgrade_source = None;
        return;
    };

    source_tower.grade = next_grade;
    source_sprite.image = gem_images.handle(source_tower.gem, next_grade);
    source_sprite.custom_size = Some(tower_sprite_size(next_grade));
    commands.entity(sacrifice_entity).despawn();
    board.towers.retain(|_, entity| *entity != sacrifice_entity);
    board.recalculate_path();
    refresh_path_markers(commands, path_markers, &board.path);
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, &source_tower);

    game.selected_tower = Some(source_entity);
    game.upgrade_source = None;
    game.message = format!(
        "Upgraded to {} {}.",
        source_tower.grade.name(),
        source_tower.gem.name()
    );
}

fn place_tower(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    path_markers: &Query<Entity, With<PathMarker>>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    gem_images: &GemImages,
    grid_pos: crate::grid::GridPos,
) {
    if board.protected_cells().contains(&grid_pos) {
        game.message =
            "Start, checkpoints, and finish stay open; build around them to shape the path."
                .to_string();
        return;
    }

    let Some(gem) = game.selected_gem() else {
        return;
    };

    let mut occupied = board.occupied_cells();
    occupied.insert(grid_pos);

    let Some(new_path) = find_complete_path(&occupied, &board.checkpoints) else {
        game.message = "That placement would block the checkpoint route.".to_string();
        return;
    };

    let tower_entity = spawn_tower(commands, grid_pos, gem, gem_images);
    board.towers.insert(grid_pos, tower_entity);
    board.path = new_path;
    refresh_path_markers(commands, path_markers, &board.path);
    clear_selection_menu(commands, menu_items);
    game.begin_countdown(gem);
    game.selected_tower = Some(tower_entity);
    game.upgrade_source = None;
    let preview_tower = chipped_tower(gem);
    spawn_selection_menu(commands, &preview_tower);
}

fn spawn_tower(
    commands: &mut Commands,
    pos: crate::grid::GridPos,
    gem: crate::gem::GemKind,
    gem_images: &GemImages,
) -> Entity {
    let tower = chipped_tower(gem);

    commands
        .spawn((
            Sprite {
                image: gem_images.handle(gem, GemGrade::Chipped),
                custom_size: Some(tower_sprite_size(GemGrade::Chipped)),
                ..default()
            },
            Transform::from_translation(grid_to_world(pos).extend(5.0)),
            tower,
            GameWorld,
        ))
        .id()
}

fn chipped_tower(gem: crate::gem::GemKind) -> Tower {
    let stats = gem.chipped_stats();
    let mut cooldown = Timer::from_seconds(stats.cooldown, TimerMode::Once);
    cooldown.set_elapsed(Duration::from_secs_f32(stats.cooldown));

    Tower {
        gem,
        grade: GemGrade::Chipped,
        damage: stats.damage,
        range: stats.range,
        cooldown,
    }
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

fn tower_at_world_position(
    world_pos: Vec2,
    towers: &Query<(Entity, &Transform), With<Tower>>,
) -> Option<Entity> {
    towers
        .iter()
        .filter_map(|(entity, transform)| {
            let tower_pos = transform.translation.truncate();
            let distance = world_pos.distance(tower_pos);
            (distance <= crate::constants::CELL_SIZE * 0.5).then_some((entity, distance))
        })
        .min_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(entity, _)| entity)
}
