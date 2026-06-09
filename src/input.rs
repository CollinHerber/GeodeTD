use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::time::Duration;

use crate::board::Board;
use crate::components::{
    ChanceBuyButton, ChancesButton, ChancesPanel, GameWorld, OfferButton, PathMarker,
    PlacementPreview, SelectionMenu, SpeedButton, StarterCandidate, StarterWall, Tower,
};
use crate::game::{AppScreen, Game, Phase, tier_grade};
use crate::gem::{
    GemEffect, GemGrade, GemKind, SpecialGem, SpecialRecipe, special_recipe_for_source,
};
use crate::gem_visual::GemImages;
use crate::grid::{grid_to_world, world_to_grid};
use crate::ui::{
    clear_selection_menu, refresh_path_markers, spawn_chances_panel, spawn_gem_info,
    spawn_keep_confirm, spawn_selection_menu, tower_sprite_size,
};

const MIN_CAMERA_SCALE: f32 = 0.45;
const MAX_CAMERA_SCALE: f32 = 5.6;
const ZOOM_STEP: f32 = 0.88;
const PIXELS_PER_SCROLL_LINE: f32 = 16.0;
const PAN_DRAG_THRESHOLD: f32 = 4.0;
const PLACEMENT_PREVIEW_Z: f32 = 6.2;

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
        let grade = game.offer_grades[index];
        game.selected_offer = Some(index);
        game.selected_tower = None;
        game.upgrade_source = None;
        clear_selection_menu(&mut commands, &menu_items);
        spawn_gem_info(&mut commands, gem, grade);
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
            let grade = game.offer_grades[offer.index];
            game.selected_offer = Some(offer.index);
            game.selected_tower = None;
            game.upgrade_source = None;
            clear_selection_menu(&mut commands, &menu_items);
            spawn_gem_info(&mut commands, gem, grade);
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

#[allow(clippy::too_many_arguments)]
pub fn handle_tower_action_clicks(
    mut commands: Commands,
    interactions: Query<
        &Interaction,
        (Changed<Interaction>, With<crate::components::UpgradeButton>),
    >,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
    mut board: ResMut<Board>,
    mut towers: Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: Query<&StarterCandidate>,
    menu_items: Query<Entity, With<SelectionMenu>>,
    gem_images: Res<GemImages>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            camera_drag.suppress_click = true;
            handle_selected_tower_action(
                &mut commands,
                &mut game,
                &mut board,
                &mut towers,
                &starters,
                &menu_items,
                &gem_images,
            );
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
        if tower.range_indicator_radius().is_some() {
            game.toggle_aura_range(tower_entity);
        }
    }
}

pub fn handle_chances_toggle(
    mut commands: Commands,
    interactions: Query<&Interaction, (Changed<Interaction>, With<ChancesButton>)>,
    mut camera_drag: ResMut<CameraDrag>,
    game: Res<Game>,
    panels: Query<Entity, With<ChancesPanel>>,
) {
    if game.screen != AppScreen::Playing {
        return;
    }

    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        camera_drag.suppress_click = true;
        if panels.iter().next().is_some() {
            for entity in &panels {
                commands.entity(entity).despawn();
            }
        } else {
            spawn_chances_panel(&mut commands);
        }
    }
}

pub fn handle_chance_buy_clicks(
    interactions: Query<(&Interaction, &ChanceBuyButton), Changed<Interaction>>,
    mut camera_drag: ResMut<CameraDrag>,
    mut game: ResMut<Game>,
) {
    if game.screen != AppScreen::Playing || game.paused {
        return;
    }

    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        camera_drag.suppress_click = true;

        let tier = button.tier;
        if game.chances.at_cap(tier) {
            game.message = format!("{} odds are already maxed.", tier_grade(tier).name());
            continue;
        }
        let cost = game.chances.next_cost();
        if game.coins < cost {
            game.message = format!("Need {} gold to raise upgrade chances.", cost);
            continue;
        }
        if let Some(spent) = game.chances.buy(tier) {
            game.coins -= spent;
            let pct = game.chances.pct(tier);
            game.message = format!(
                "{} starter odds raised to {}%.",
                tier_grade(tier).name(),
                pct
            );
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
    mut towers: Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
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

pub fn update_placement_preview(
    mut commands: Commands,
    game: Res<Game>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
    board: Res<Board>,
    previews: Query<Entity, With<PlacementPreview>>,
    gem_images: Res<GemImages>,
) {
    for entity in previews.iter() {
        commands.entity(entity).despawn();
    }

    if game.screen != AppScreen::Playing || game.paused || game.phase != Phase::Build {
        return;
    }

    let Some(gem) = game.selected_gem() else {
        return;
    };
    let grade = game.selected_offer_grade().unwrap_or(GemGrade::Chipped);

    let Some(world_pos) = cursor_world_position(&windows, &camera) else {
        return;
    };
    let Some(grid_pos) = world_to_grid(world_pos) else {
        return;
    };

    let mut occupied = board.occupied_cells();
    let valid = !board.protected_cells().contains(&grid_pos) && !occupied.contains(&grid_pos) && {
        occupied.insert(grid_pos);
        board.path_with_blocked(&occupied).is_some()
    };

    let color = if valid {
        Color::srgba(0.70, 1.0, 0.88, 0.34)
    } else {
        Color::srgba(1.0, 0.18, 0.16, 0.32)
    };

    commands.spawn((
        Sprite::from_color(color, Vec2::splat(crate::constants::CELL_SIZE * 0.92)),
        Transform::from_translation(grid_to_world(grid_pos).extend(PLACEMENT_PREVIEW_Z - 0.1)),
        PlacementPreview,
        GameWorld,
    ));
    commands.spawn((
        Sprite {
            image: gem_images.handle(gem, grade),
            custom_size: Some(tower_sprite_size(grade) * 0.9),
            color: Color::srgba(1.0, 1.0, 1.0, if valid { 0.58 } else { 0.32 }),
            ..default()
        },
        Transform::from_translation(grid_to_world(grid_pos).extend(PLACEMENT_PREVIEW_Z)),
        PlacementPreview,
        GameWorld,
    ));
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
    mut towers: Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
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

    if let Some(tower_entity) = tower_at_world_position(world_pos, &towers) {
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

enum SelectedTowerAction {
    RegularUpgrade,
    CombineSpecial(SpecialRecipe),
    UpgradeSpecial(SpecialGem),
}

#[allow(clippy::too_many_arguments)]
fn handle_selected_tower_action(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    gem_images: &GemImages,
) {
    let Some(source) = game.selected_tower else {
        return;
    };

    let Some(action) = selected_tower_action(source, game, towers) else {
        return;
    };

    match action {
        SelectedTowerAction::RegularUpgrade => {
            begin_upgrade_selection(commands, game, towers, starters, menu_items);
        }
        SelectedTowerAction::CombineSpecial(recipe) => {
            combine_special(
                commands, game, board, towers, starters, menu_items, gem_images, source, recipe,
            );
        }
        SelectedTowerAction::UpgradeSpecial(special) => {
            upgrade_special(
                commands, game, towers, menu_items, gem_images, source, special,
            );
        }
    }
}

fn selected_tower_action(
    source: Entity,
    game: &mut Game,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
) -> Option<SelectedTowerAction> {
    let Ok((_, _, tower, _)) = towers.get_mut(source) else {
        game.selected_tower = None;
        return None;
    };

    if let Some(special) = tower.special
        && special.upgrade().is_some()
    {
        Some(SelectedTowerAction::UpgradeSpecial(special))
    } else if let Some(recipe) = special_recipe_for_source(tower.gem, tower.grade) {
        Some(SelectedTowerAction::CombineSpecial(recipe))
    } else {
        Some(SelectedTowerAction::RegularUpgrade)
    }
}

fn begin_upgrade_selection(
    commands: &mut Commands,
    game: &mut Game,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
) {
    let Some(source) = game.selected_tower else {
        return;
    };

    let Some((gem, grade)) = ({
        let Ok((_, _, tower, _)) = towers.get_mut(source) else {
            game.selected_tower = None;
            return;
        };

        if !tower.can_regular_upgrade() {
            game.message = "That tower cannot be upgraded with a duplicate.".to_string();
            return;
        }

        Some((tower.gem, tower.grade))
    }) else {
        return;
    };

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
    gem: GemKind,
    grade: GemGrade,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
) -> bool {
    towers.iter_mut().any(|(entity, _, tower, _)| {
        entity != source && starters.get(entity).is_err() && tower.is_basic(gem, grade)
    })
}

#[allow(clippy::too_many_arguments)]
fn combine_special(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    gem_images: &GemImages,
    source: Entity,
    recipe: SpecialRecipe,
) {
    let Some(components) = special_component_entities(source, recipe, towers, starters) else {
        game.message = format!(
            "{} needs {}.",
            recipe.result.name(),
            recipe.ingredient_summary()
        );
        game.upgrade_source = None;
        return;
    };

    {
        let Ok((_, _, mut source_tower, mut source_sprite)) = towers.get_mut(source) else {
            game.message = format!(
                "{} recipe source is no longer available.",
                recipe.result.name()
            );
            return;
        };

        source_tower.become_special(recipe.result);
        source_sprite.image = gem_images.special_handle(recipe.result);
        source_sprite.custom_size = Some(tower_sprite_size(recipe.result.sprite_grade()));
    }

    for component in components {
        sacrifice_tower_to_wall(commands, game, board, component);
    }

    clear_selection_menu(commands, menu_items);
    if let Ok((_, _, source_tower, _)) = towers.get_mut(source) {
        spawn_selection_menu(commands, &source_tower);
    }
    game.selected_tower = Some(source);
    game.selected_offer = None;
    game.upgrade_source = None;
    game.message = format!(
        "Combined {} into {}.",
        recipe.ingredient_summary(),
        recipe.result.name()
    );
}

fn special_component_entities(
    source: Entity,
    recipe: SpecialRecipe,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    starters: &Query<&StarterCandidate>,
) -> Option<Vec<Entity>> {
    let mut found = vec![None; recipe.components.len()];

    for (entity, _, tower, _) in towers.iter_mut() {
        if entity == source || starters.get(entity).is_ok() {
            continue;
        }

        for (index, (gem, grade)) in recipe.components.iter().enumerate() {
            if found[index].is_none() && tower.is_basic(*gem, *grade) {
                found[index] = Some(entity);
                break;
            }
        }
    }

    found.into_iter().collect()
}

fn upgrade_special(
    commands: &mut Commands,
    game: &mut Game,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    gem_images: &GemImages,
    source: Entity,
    special: SpecialGem,
) {
    let Some(next) = special.upgrade() else {
        game.message = format!("{} has no further upgrade.", special.name());
        return;
    };
    let Some(cost) = special.upgrade_cost() else {
        game.message = format!("{} has no upgrade cost configured.", special.name());
        return;
    };
    if game.coins < cost {
        game.message = format!("Need {} gold to upgrade {}.", cost, special.name());
        return;
    }

    let source_position = {
        let Ok((_, transform, mut tower, mut sprite)) = towers.get_mut(source) else {
            game.selected_tower = None;
            return;
        };
        if tower.special != Some(special) {
            game.message = format!("Only {} can upgrade to {}.", special.name(), next.name());
            return;
        }

        game.coins -= cost;
        let source_position = transform.translation.truncate();
        tower.become_special(next);
        sprite.image = gem_images.special_handle(next);
        sprite.custom_size = Some(tower_sprite_size(next.sprite_grade()));
        source_position
    };

    let mut boosted = 0;
    if let Some((damage_bonus, boost_radius)) = special_damage_boost(next.effect()) {
        for (entity, transform, mut other_tower, _) in towers.iter_mut() {
            if entity == source || other_tower.black_opal_boosted {
                continue;
            }
            if other_tower.effect().ignores_opal_effects() {
                continue;
            }
            if transform.translation.truncate().distance(source_position) <= boost_radius {
                if other_tower.yellow_sapphire_boosted {
                    if next != SpecialGem::MysticBlackOpal {
                        continue;
                    }
                    other_tower.damage /= 1.0 + SpecialGem::STAR_YELLOW_SAPPHIRE_DAMAGE_BOOST;
                    other_tower.yellow_sapphire_boosted = false;
                }
                other_tower.damage *= 1.0 + damage_bonus;
                if next == SpecialGem::MysticBlackOpal {
                    other_tower.black_opal_boosted = true;
                } else if next == SpecialGem::StarYellowSapphire {
                    other_tower.yellow_sapphire_boosted = true;
                }
                boosted += 1;
            }
        }
    }

    clear_selection_menu(commands, menu_items);
    if let Ok((_, _, tower, _)) = towers.get_mut(source) {
        spawn_selection_menu(commands, &tower);
    }
    game.selected_tower = Some(source);
    game.selected_offer = None;
    game.upgrade_source = None;
    game.message = match next {
        SpecialGem::MysticBlackOpal => format!(
            "Upgraded to Mystic Black Opal. Boosted {} existing tower{} by 40%.",
            boosted,
            if boosted == 1 { "" } else { "s" }
        ),
        SpecialGem::StarYellowSapphire => format!(
            "Upgraded to Star Yellow Sapphire. Boosted {} existing tower{} by 5%.",
            boosted,
            if boosted == 1 { "" } else { "s" }
        ),
        _ => format!("Upgraded to {}.", next.name()),
    };
}

fn special_damage_boost(effect: GemEffect) -> Option<(f32, f32)> {
    match effect {
        GemEffect::DamageBoost {
            damage_bonus,
            range,
        } => Some((damage_bonus, range)),
        GemEffect::SlowSplashBoost {
            damage_bonus,
            boost_range,
            ..
        } => Some((damage_bonus, boost_range)),
        _ => None,
    }
}

fn select_tower(
    commands: &mut Commands,
    game: &mut Game,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    tower_entity: Entity,
) {
    let Ok((_, _, tower, _)) = towers.get(tower_entity) else {
        return;
    };

    game.selected_tower = Some(tower_entity);
    game.selected_offer = None;
    game.upgrade_source = None;
    game.keep_candidate = None;
    game.message = format!("Selected {}.", tower.display_name());
    clear_selection_menu(commands, menu_items);
    spawn_selection_menu(commands, tower);
}

/// Picks (but doesn't commit) a starter to keep once all five are placed. The
/// choice is highlighted and a Confirm button is shown; Ctrl+click skips this.
fn select_keep_candidate(
    commands: &mut Commands,
    game: &mut Game,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    tower_entity: Entity,
) {
    let Ok((_, _, tower, _)) = towers.get(tower_entity) else {
        return;
    };
    let gem = tower.gem;
    let grade = tower.grade;

    game.keep_candidate = Some(tower_entity);
    game.selected_tower = None;
    game.selected_offer = None;
    game.upgrade_source = None;
    game.message = format!(
        "Keep the {} starter? Press Confirm (or Ctrl+click a starter to keep instantly).",
        gem.name()
    );
    clear_selection_menu(commands, menu_items);
    spawn_keep_confirm(commands, gem, grade);
}

fn inspect_starter_candidate(
    commands: &mut Commands,
    game: &mut Game,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    tower_entity: Entity,
) {
    let Ok((_, _, tower, _)) = towers.get(tower_entity) else {
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
    spawn_gem_info(commands, tower.gem, tower.grade);
}

#[allow(clippy::too_many_arguments)]
fn keep_starter(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    path_markers: &Query<Entity, With<PathMarker>>,
    menu_items: &Query<Entity, With<SelectionMenu>>,
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
    kept_entity: Entity,
) {
    let Ok((_, _, kept_tower, _)) = towers.get(kept_entity) else {
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
    towers: &mut Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
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
            (_, _, mut source_tower, mut source_sprite),
            (_, _, sacrifice_tower, _),
        ],
    ) = towers.get_many_mut([source_entity, sacrifice_entity])
    else {
        game.message = "That upgrade target is no longer available.".to_string();
        game.upgrade_source = None;
        return;
    };

    if !sacrifice_tower.is_basic(source_tower.gem, source_tower.grade) {
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
    sacrifice_tower_to_wall(commands, game, board, sacrifice_entity);
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

fn sacrifice_tower_to_wall(
    commands: &mut Commands,
    game: &mut Game,
    board: &mut Board,
    tower_entity: Entity,
) {
    if let Some(pos) = grid_pos_for_entity(board, tower_entity) {
        commands.entity(tower_entity).despawn();
        board.towers.remove(&pos);
        game.shown_aura_ranges.remove(&tower_entity);
        board.walls.insert(pos);
        spawn_starter_wall(commands, pos);
    } else {
        commands.entity(tower_entity).despawn();
        game.shown_aura_ranges.remove(&tower_entity);
    }
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
    let grade = game.offer_grades[offer_index];

    if game.placed_starters[offer_index].is_some() {
        game.message = "That chipped gem has already been placed.".to_string();
        game.selected_offer = None;
        return;
    }

    let mut occupied = board.occupied_cells();
    occupied.insert(grid_pos);

    let Some(new_path) = board.path_with_blocked(&occupied) else {
        game.message = "That placement would block the checkpoint route.".to_string();
        return;
    };

    let tower_entity = spawn_tower(
        commands,
        grid_pos,
        gem,
        grade,
        gem_images,
        Some(offer_index),
    );
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
        spawn_gem_info(commands, gem, grade);
    } else {
        game.message = format!(
            "Placed starter {} of {}. Select and place the next chipped gem.",
            placed,
            crate::constants::OFFER_COUNT
        );
        spawn_gem_info(commands, gem, grade);
    }
}

fn spawn_tower(
    commands: &mut Commands,
    pos: crate::grid::GridPos,
    gem: crate::gem::GemKind,
    grade: GemGrade,
    gem_images: &GemImages,
    starter_index: Option<usize>,
) -> Entity {
    let tower = tower_for_grade(gem, grade);
    let mut entity = commands.spawn((
        Sprite {
            image: gem_images.handle(gem, grade),
            custom_size: Some(tower_sprite_size(grade)),
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

fn tower_for_grade(gem: crate::gem::GemKind, grade: GemGrade) -> Tower {
    let stats = gem.chipped_stats();
    let mut cooldown = Timer::from_seconds(stats.cooldown, TimerMode::Once);
    cooldown.set_elapsed(Duration::from_secs_f32(stats.cooldown));

    Tower {
        gem,
        grade,
        special: None,
        damage: stats.damage,
        range: stats.range,
        cooldown,
        black_opal_boosted: false,
        yellow_sapphire_boosted: false,
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
    towers: &Query<(Entity, &Transform, &mut Tower, &mut Sprite)>,
) -> Option<Entity> {
    towers
        .iter()
        .filter_map(|(entity, transform, _, _)| {
            let tower_pos = transform.translation.truncate();
            let distance = world_pos.distance(tower_pos);
            (distance <= crate::constants::CELL_SIZE * 0.5).then_some((entity, distance))
        })
        .min_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(entity, _)| entity)
}
