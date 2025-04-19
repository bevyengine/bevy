//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    dev_tools::fps_overlay::FpsOverlayPlugin,
    prelude::*,
    sprite::{TilemapChunk, TilemapChunkIndices},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            FpsOverlayPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_tilemap, update_tilemap))
        .run();
}

#[derive(Resource)]
struct LoadingTileset {
    is_loaded: bool,
    handle: Handle<Image>,
}

#[derive(Component, Deref, DerefMut)]
struct UpdateTimer(Timer);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(LoadingTileset {
        is_loaded: false,
        handle: asset_server.load("textures/array_texture.png"),
    });

    commands.spawn(Camera2d);
}

fn spawn_tilemap(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_tileset: ResMut<LoadingTileset>,
    mut images: ResMut<Assets<Image>>,
) {
    if loading_tileset.is_loaded
        || !asset_server
            .load_state(loading_tileset.handle.id())
            .is_loaded()
    {
        return;
    }
    loading_tileset.is_loaded = true;
    let image = images.get_mut(&loading_tileset.handle).unwrap();
    image.reinterpret_stacked_2d_as_array(4);

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let chunk_size = UVec2::splat(64);
    let tile_display_size = UVec2::splat(8);
    let indices: Vec<Option<u32>> = (0..chunk_size.x * chunk_size.y)
        .map(|_| rng.gen_range(0..5))
        .map(|i| if i == 0 { None } else { Some(i - 1) })
        .collect();

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: loading_tileset.handle.clone(),
        },
        TilemapChunkIndices(indices),
        UpdateTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
}

fn update_tilemap(time: Res<Time>, mut query: Query<(&mut TilemapChunkIndices, &mut UpdateTimer)>) {
    for (mut indices, mut timer) in query.iter_mut() {
        timer.tick(time.delta());

        if timer.just_finished() {
            let mut rng = ChaCha8Rng::from_entropy();
            for _ in 0..50 {
                let index = rng.gen_range(0..indices.len());
                indices[index] = Some(rng.gen_range(0..5));
            }
        }
    }
}
