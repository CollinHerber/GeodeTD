use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::time::Duration;

use crate::board::{Board, find_complete_path};
use crate::components::{
    GameWorld, OfferButton, PathMarker, SelectionMenu, SpeedButton, StarterCandidate, StarterWall,
    Tower,
};
use crate::game::{AppScreen, Game, Phase};
use crate::gem::GemGrade;
use crate::gem_visual::GemImages;
use crate::grid::{grid_to_world, world_to_grid};
use crate::ui::{
    clear_selection_menu, refresh_path_markers, spawn_gem_info, spawn_keep_confirm,
    spawn_selection_menu, tower_sprite_size,
};

const MIN_CAMERA_SCALE: f32 = 0.45;
const MAX_CAMERA_SCALE: f32 = 5.6;
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
    if game.screen != AppScreen::Playing || game.paused {
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
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Build {
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
        game.selected_offer = Some(index);
        game.selected_tower = None;
        game.upgrade_source = None;
        clear_selection_menu(&mut commands, &menu_items);
        spawn_gem_info(&mut commands, gem);
    }
}

pub fn handle_offer_clicks(
    mut commands: Commands,
    interactions: Query<(&Interaction, &OfferButton), Changed<Interaction>>,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    menu_items: Query<Entity, With<SelectionMenu>>,
) {
    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Build {
        return;
    }

    for (interaction, offer) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        camera_drag.suppress_click = true;
        if let Some(gem) = game.offers[offer.index] {
            game.selected_offer = Some(offer.index);
            game.selected_tower = None;
            game.upgrade_source = None;
            clear_selection_menu(&mut commands, &menu_items);
            spawn_gem_info(&mut commands, gem);
        }
    }
}

pub fn handle_speed_clicks(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SpeedButton>)>,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            camera_drag.suppress_click = true;
            game.cycle_speed();
        }
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
    towers: Query<(Entity, &Tower)>,
    starters: Query<&StarterCandidate>,
    menu_items: Query<Entity, With<SelectionMenu>>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            camera_drag.suppress_click = true;
            begin_upgrade_selection(&mut commands, &mut game, &towers, &starters, &menu_items);
        }
    }
}

pub fn handle_show_range_clicks(
    interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<crate::components::ShowRangeButton>,
        ),
    >,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    towers: Query<&Tower>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        camera_drag.suppress_click = true;
        let Some(tower_entity) = game.selected_tower else {
            continue;
        };
        let Ok(tower) = towers.get(tower_entity) else {
            continue;
        };
        if matches!(
            tower.gem.effect(tower.grade),
            crate::gem::GemEffect::Haste { .. }
        ) {
            game.toggle_aura_range(tower_entity);
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_keep_confirm_clicks(
    mut commands: Commands,
    interactions: Query<
        &Interaction,
        (
            Changed<Interaction>,
            With<crate::components::ConfirmKeepButton>,
        ),
    >,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    path_markers: Query<Entity, With<PathMarker>>,
    menu_items: Query<Entity, With<SelectionMenu>>,
    mut towers: Query<(Entity, &mut Tower, &mut Sprite)>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        camera_drag.suppress_click = true;
        let Some(candidate) = game.keep_candidate else {
            continue;
        };
        keep_starter(
            &mut commands,
            &mut game,
            &mut board,
            &path_markers,
            &menu_items,
            &mut towers,
            candidate,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn place_or_select(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    path_markers: Query<Entity, With<PathMarker>>,
    menu_items: Query<Entity, With<SelectionMenu>>,
    mut towers: Query<(Entity, &mut Tower, &mut Sprite)>,
    tower_positions: Query<(Entity, &Transform), With<Tower>>,
    starter_candidates: Query<&StarterCandidate>,
    gem_images: Res<GemImages>,
) {
    if game.screen != AppScreen::Playing || game.paused || !buttons.just_released(MouseButton::Left)
    {
        return;
    }

    let instant_keep = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight);

    if camera_drag.suppress_click {
        camera_drag.suppress_click = false;
        return;
    }

    let Some(world_pos) = cursor_world_position(&windows, &camera) else {
        return;
    };

    if let Some(tower_entity) = tower_at_world_position(world_pos, &tower_positions) {
        let is_starter_candidate = starter_candidates.get(tower_entity).is_ok();
        // Upgrade mode takes precedence (in any phase, so upgrades work mid-wave) so
        // a click on a matching tower is consumed instead of inspected/kept.
        if game.upgrade_source.is_some() {
            complete_upgrade(
                &mut commands,
                &mut game,
                &mut board,
                &menu_items,
                &mut towers,
                &starter_candidates,
                &gem_images,
                tower_entity,
            );
        } else if game.phase == Phase::Build && game.all_starters_placed() && is_starter_candidate {
            // Default: pick a starter and wait for Confirm. Ctrl+click keeps now.
            if instant_keep {
                keep_starter(
                    &mut commands,
                    &mut game,
                    &mut board,
                    &path_markers,
                    &menu_items,
                    &mut towers,
                    tower_entity,
                );
            } else {
                select_keep_candidate(&mut commands, &mut game, &menu_items, &towers, tower_entity);
            }
        } else if game.phase == Phase::Build && is_starter_candidate {
            inspect_starter_candidate(&mut commands, &mut game, &menu_items, &towers, tower_entity);
        } else {
            select_tower(&mut commands, &mut game, &menu_items, &towers, tower_entity);
        }
        return;
    }

    let had_tower_display = game.selected_tower.is_some()
        || game.upgrade_source.is_some()
        || game.keep_candidate.is_some();
    if had_tower_display {
        game.selected_tower = None;
        game.upgrade_source = None;
        game.keep_candidate = None;
        clear_selection_menu(&mut commands, &menu_items);
    }

    if game.phase != Phase::Build {
        return;
    }

    let Some(grid_pos) = world_to_grid(world_pos) else {
        game.selected_offer = None;
        clear_selection_menu(&mut commands, &menu_items);
        return;
    };

    if game.upgrade_source.is_some() {
        game.message = "Select a matching duplicate tower to sacrifice.".to_string();
        return;
    }

    if had_tower_display {
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
    towers: &Query<(Entity, &Tower)>,
    starters: &Query<&StarterCandidate>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
) {
    let Some(source) = game.selected_tower else {
        return;
    };
    let Some((gem, grade)) = towers
        .get(source)
        .ok()
        .map(|(_, tower)| (tower.gem, tower.grade))
    else {
        game.selected_tower = None;
        return;
    };

    if grade.next().is_none() {
        game.message = "That tower is already Perfect.".to_string();
        return;
    }

    if !upgrade_match_exists(source, gem, grade, towers, starters) {
        game.message = format!(
            "Need another placed {} {} to upgrade.",
            grade.name(),
            gem.name()
        );
        return;
    }

    game.upgrade_source = Some(source);
    game.message = format!(
        "Click a highlighted {} {} to sacrifice.",
        grade.name(),
        gem.name()
    );
    clear_selection_menu(commands, menu_items);
}

/// Whether another placed (non-starter) tower of the same gem and grade exists to
/// feed an upgrade of `source`.
fn upgrade_match_exists(
    source: Entity,
    gem: crate::gem::GemKind,
    grade: GemGrade,
    towers: &Query<(Entity, &Tower)>,
    starters: &Query<&StarterCandidate>,
) -> bool {
    towers.iter().any(|(entity, tower)| {
        entity != source
            && starters.get(entity).is_err()
            && tower.gem == gem
            && tower.grade == grade
    })
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
    game.selected_offer = None;
    game.upgrade_source = None;
    game.keep_candidate = None;
    game.message = format!("Selected {} {}.", tower.grade.name(), tower.gem.name());
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, tower);
}

/// Picks (but doesn't commit) a starter to keep once all five are placed. The
/// choice is highlighted and a Confirm button is shown; Ctrl+click skips this.
fn select_keep_candidate(
    commands: &mut Commands,
    game: &mut Game,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &Query<(Entity, &mut Tower, &mut Sprite)>,
    tower_entity: Entity,
) {
    let Ok((_, tower, _)) = towers.get(tower_entity) else {
        return;
    };
    let gem = tower.gem;

    game.keep_candidate = Some(tower_entity);
    game.selected_tower = None;
    game.selected_offer = None;
    game.upgrade_source = None;
    game.message = format!(
        "Keep the {} starter? Press Confirm (or Ctrl+click a starter to keep instantly).",
        gem.name()
    );
    clear_selection_menu(commands, menu_items);
    spawn_keep_confirm(commands, gem);
}

fn inspect_starter_candidate(
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
    game.selected_offer = None;
    game.upgrade_source = None;
    game.message = format!(
        "Placed starter {} of {}. Place the rest before choosing one.",
        game.placed_starter_count(),
        crate::constants::OFFER_COUNT
    );
    clear_selection_menu(commands, menu_items);
    spawn_gem_info(commands, tower.gem);
}

#[allow(clippy::too_many_arguments)]
fn keep_starter(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    path_markers: &Query<Entity, With<PathMarker>>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &mut Query<(Entity, &mut Tower, &mut Sprite)>,
    kept_entity: Entity,
) {
    let Ok((_, kept_tower, _)) = towers.get(kept_entity) else {
        game.message = "That starter is no longer available.".to_string();
        return;
    };
    let kept_gem = kept_tower.gem;

    let starters = game.placed_starters;
    for starter in starters.iter().flatten().copied() {
        if starter == kept_entity {
            commands.entity(starter).remove::<StarterCandidate>();
            continue;
        }

        if let Some(pos) = grid_pos_for_entity(board, starter) {
            commands.entity(starter).despawn();
            board.towers.remove(&pos);
            game.shown_aura_ranges.remove(&starter);
            board.walls.insert(pos);
            spawn_starter_wall(commands, pos);
        }
    }

    board.recalculate_path();
    refresh_path_markers(commands, path_markers, &board.path, game.show_path_overlay);
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, kept_tower);
    game.begin_countdown(kept_gem);
    game.selected_tower = Some(kept_entity);
    game.upgrade_source = None;
    game.keep_candidate = None;
}

#[allow(clippy::too_many_arguments)]
fn complete_upgrade(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &mut Query<(Entity, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
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

    if starters.get(sacrifice_entity).is_ok() {
        game.message = "Sacrifice a placed tower, not a starter you haven't kept.".to_string();
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

    // The sacrificed tower turns into a wall. Its cell stays occupied, so the path
    // is unchanged and in-flight enemies are untouched — which is what makes
    // upgrading safe in the middle of a wave.
    if let Some(pos) = grid_pos_for_entity(board, sacrifice_entity) {
        commands.entity(sacrifice_entity).despawn();
        board.towers.remove(&pos);
        game.shown_aura_ranges.remove(&sacrifice_entity);
        board.walls.insert(pos);
        spawn_starter_wall(commands, pos);
    } else {
        commands.entity(sacrifice_entity).despawn();
        game.shown_aura_ranges.remove(&sacrifice_entity);
    }
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, &source_tower);

    game.selected_tower = Some(source_entity);
    game.selected_offer = None;
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
        game.message = "Select a chipped gem before placing a tower.".to_string();
        return;
    };
    let Some(offer_index) = game.selected_offer else {
        game.message = "Select a chipped gem before placing a tower.".to_string();
        return;
    };

    if game.placed_starters[offer_index].is_some() {
        game.message = "That chipped gem has already been placed.".to_string();
        game.selected_offer = None;
        return;
    }

    let mut occupied = board.occupied_cells();
    occupied.insert(grid_pos);

    let Some(new_path) = find_complete_path(&occupied, &board.checkpoints) else {
        game.message = "That placement would block the checkpoint route.".to_string();
        return;
    };

    let tower_entity = spawn_tower(commands, grid_pos, gem, gem_images, Some(offer_index));
    board.towers.insert(grid_pos, tower_entity);
    board.path = new_path;
    refresh_path_markers(commands, path_markers, &board.path, game.show_path_overlay);
    clear_selection_menu(commands, menu_items);
    game.selected_tower = Some(tower_entity);
    game.selected_offer = None;
    game.offers[offer_index] = None;
    game.placed_starters[offer_index] = Some(tower_entity);
    game.upgrade_source = None;

    let placed = game.placed_starter_count();
    if game.all_starters_placed() {
        game.message =
            "All five placed. Click one to select, then Confirm (Ctrl+click to keep instantly)."
                .to_string();
        let preview_tower = chipped_tower(gem);
        spawn_gem_info(commands, preview_tower.gem);
    } else {
        game.message = format!(
            "Placed starter {} of {}. Select and place the next chipped gem.",
            placed,
            crate::constants::OFFER_COUNT
        );
        spawn_gem_info(commands, gem);
    }
}

fn spawn_tower(
    commands: &mut Commands,
    pos: crate::grid::GridPos,
    gem: crate::gem::GemKind,
    gem_images: &GemImages,
    starter_index: Option<usize>,
) -> Entity {
    let tower = chipped_tower(gem);
    let mut entity = commands.spawn((
        Sprite {
            image: gem_images.handle(gem, GemGrade::Chipped),
            custom_size: Some(tower_sprite_size(GemGrade::Chipped)),
            ..default()
        },
        Transform::from_translation(grid_to_world(pos).extend(5.0)),
        tower,
        GameWorld,
    ));

    if starter_index.is_some() {
        entity.insert(StarterCandidate);
    }

    entity.id()
}

fn spawn_starter_wall(commands: &mut Commands, pos: crate::grid::GridPos) {
    commands.spawn((
        Sprite::from_color(
            Color::srgb(0.35, 0.36, 0.34),
            Vec2::splat(crate::constants::CELL_SIZE * 0.78),
        ),
        Transform::from_translation(grid_to_world(pos).extend(4.0)),
        StarterWall,
        GameWorld,
    ));
}

fn grid_pos_for_entity(board: &Board, entity: Entity) -> Option<crate::grid::GridPos> {
    board
        .towers
        .iter()
        .find_map(|(pos, tower)| (*tower == entity).then_some(*pos))
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
