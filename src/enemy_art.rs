use bevy::prelude::*;

#[derive(Resource)]
pub struct EnemyArt {
    pub gemling_image: Handle<Image>,
    pub gemling_layout: Handle<TextureAtlasLayout>,
}

impl FromWorld for EnemyArt {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let gemling_image = asset_server.load("enemies/gemling-walk.png");

        let mut layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        let gemling_layout = layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(192),
            8,
            1,
            None,
            None,
        ));

        Self {
            gemling_image,
            gemling_layout,
        }
    }
}
