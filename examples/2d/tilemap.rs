//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    color::palettes::tailwind::RED_400,
    image::{ImageArrayLayout, ImageLoaderSettings},
    prelude::*,
    sprite::{CommandsTilemapExt, TileStorage, Tilemap, TilemapQuery},
    sprite_render::{TileRenderData, TilemapChunkRenderData, TilemapRenderData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, set_tile)
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct UpdateTimer(Timer);

#[derive(Resource, Deref, DerefMut)]
struct SeededRng(ChaCha8Rng);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let chunk_size = UVec2::splat(16);
    let tile_display_size = UVec2::splat(32);

    commands.spawn((
        Transform::default(),
        Visibility::default(),
        Tilemap::new(chunk_size, tile_display_size),
        TilemapRenderData {
            tileset: assets.load_with_settings(
                "textures/array_texture.png",
                |settings: &mut ImageLoaderSettings| {
                    // The tileset texture is expected to be an array of tile textures, so we tell the
                    // `ImageLoader` that our texture is composed of 4 stacked tile images.
                    settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 4 });
                },
            ),
            ..default()
        },
    ));

    commands.spawn(Camera2d);

    commands.insert_resource(SeededRng(rng));
}

fn set_tile(
    mut commands: Commands,
    mut clicks: MessageReader<Pointer<Click>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    map: Single<Entity, With<Tilemap>>,
    mut tiles: TilemapQuery<&mut TileRenderData>,
) {
    let (camera, camera_transform) = *camera_query;
    let map = *map;

    let mut tiles = tiles.get_map_mut(map).unwrap();

    for click in clicks.read() {
        if let Ok(tile_coord) =
            camera.viewport_to_world_2d(camera_transform, click.pointer_location.position)
        {
            let tile_coord = tiles.map.get_tile_coord(tile_coord);
            info!("{}", tile_coord);
            if let Some(tile) = tiles.get_at_mut(tile_coord) {
                tile.tileset_index = (tile.tileset_index + 1) % 4;
            } else {
                commands.set_tile(
                    map,
                    tile_coord,
                    Some(TileRenderData {
                        tileset_index: 0,
                        ..Default::default()
                    }),
                );
            }
        }
    }
}
