//! Shows a tilemap chunk rendered with a single draw call.

use core::num::NonZero;

use bevy::{
    image::ImageLoaderSettings,
    prelude::*,
    sprite::{TileData, TilemapChunk, TilemapChunkTileData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, update_tilemap)
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

    let chunk_size = UVec2::splat(64);
    let tile_display_size = UVec2::splat(8);
    let tile_data: Vec<Option<TileData>> = (0..chunk_size.element_product())
        .map(|_| rng.random_range(0..5))
        .map(|i| {
            if i == 0 {
                None
            } else {
                Some(TileData::from_tileset_index(i - 1))
            }
        })
        .collect();

    let layers = 4;
    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load_with_settings(
                "textures/array_texture.png",
                move |settings: &mut ImageLoaderSettings| {
                    settings.layers = NonZero::new(layers);
                },
            ),
            ..Default::default()
        },
        TilemapChunkTileData(tile_data),
        UpdateTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    commands.spawn(Camera2d);

    commands.insert_resource(SeededRng(rng));
}

fn update_tilemap(
    time: Res<Time>,
    mut query: Query<(&mut TilemapChunkTileData, &mut UpdateTimer)>,
    mut rng: ResMut<SeededRng>,
) {
    for (mut tile_data, mut timer) in query.iter_mut() {
        timer.tick(time.delta());

        if timer.just_finished() {
            for _ in 0..50 {
                let index = rng.random_range(0..tile_data.len());
                tile_data[index] = Some(TileData::from_tileset_index(rng.random_range(0..5)));
            }
        }
    }
}
