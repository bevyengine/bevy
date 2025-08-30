//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    prelude::*,
    sprite_render::{TileData, TilemapChunk, TilemapChunkTileData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_tileset_image, update_tilemap))
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

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load("textures/array_texture.png"),
            ..default()
        },
        TilemapChunkTileData(tile_data),
        UpdateTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    commands.spawn(Camera2d);

    commands.insert_resource(SeededRng(rng));
}

fn update_tileset_image(
    chunk_query: Single<&TilemapChunk>,
    mut events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    let chunk = *chunk_query;
    for event in events.read() {
        if event.is_loaded_with_dependencies(chunk.tileset.id()) {
            let image = images.get_mut(&chunk.tileset).unwrap();
            image.reinterpret_stacked_2d_as_array(4);
        }
    }
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
