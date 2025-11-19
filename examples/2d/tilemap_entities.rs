//! Shows a tilemap chunk rendered with a single draw call.

use std::time::Duration;

use bevy::{
    color::palettes::tailwind::RED_400,
    image::{ImageArrayLayout, ImageLoaderSettings},
    prelude::*,
    sprite::{CommandsTilemapExt, DespawnOnRemove, InMap, TileCoord, TileStorage, Tilemap},
    sprite_render::{TileRenderData, TilemapChunkRenderData, TilemapRenderData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, (spin_tilemap, update_tilemap, remove_dead_tiles))
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct DeathTimer(Timer);

#[derive(Resource, Deref, DerefMut)]
struct SeededRng(ChaCha8Rng);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let chunk_size = UVec2::splat(16);
    let tile_display_size = UVec2::splat(8);

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

fn update_tilemap(
    map: Single<Entity, With<Tilemap>>,
    mut commands: Commands,
    mut rng: ResMut<SeededRng>,
) {
    let map = *map;
    for _ in 0..rng.random_range(1..=20) {
        let x = rng.random_range(-64..=64);
        let y = rng.random_range(-64..=64);

        commands.spawn((
            InMap(map),
            TileCoord(IVec2::new(x, y)),
            TileRenderData {
                tileset_index: rng.random_range(0..4),
                ..Default::default()
            },
            DeathTimer(Timer::new(Duration::from_secs(3), TimerMode::Once)),
            DespawnOnRemove,
        ));
    }
}

fn spin_tilemap(time: Res<Time>, mut map: Single<&mut Transform, With<Tilemap>>) {
    map.rotate_z(time.delta_secs() * 0.1);
}

fn remove_dead_tiles(
    mut map: Query<(Entity, &mut DeathTimer)>,
    time: Res<Time>,
    mut commands: Commands,
) {
    for (id, mut timer) in map.iter_mut() {
        if timer.tick(time.delta()).is_finished() {
            commands.entity(id).despawn();
        }
    }
}
