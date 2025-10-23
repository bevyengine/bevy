//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    color::palettes::tailwind::RED_400,
    prelude::*,
    sprite_render::{TileData, TilemapChunk, TilemapChunkTileData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (setup, spawn_fake_player).chain())
        .add_systems(
            Update,
            (update_tileset_image, update_tilemap, move_player, log_tile),
        )
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

#[derive(Component)]
struct MovePlayer;

fn spawn_fake_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    chunk: Single<&TilemapChunk>,
) {
    let mut transform = chunk.calculate_tile_transform(UVec2::new(0, 0));
    transform.translation.z = 1.;

    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(8., 8.))),
        MeshMaterial2d(materials.add(Color::from(RED_400))),
        transform,
        MovePlayer,
    ));

    let mut transform = chunk.calculate_tile_transform(UVec2::new(5, 6));
    transform.translation.z = 1.;

    // second "player" to visually test a non-zero position
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(8., 8.))),
        MeshMaterial2d(materials.add(Color::from(RED_400))),
        transform,
    ));
}

fn move_player(
    mut player: Single<&mut Transform, With<MovePlayer>>,
    time: Res<Time>,
    chunk: Single<&TilemapChunk>,
) {
    let t = (ops::sin(time.elapsed_secs()) + 1.) / 2.;

    let origin = chunk
        .calculate_tile_transform(UVec2::new(0, 0))
        .translation
        .x;
    let destination = chunk
        .calculate_tile_transform(UVec2::new(63, 0))
        .translation
        .x;

    player.translation.x = origin.lerp(destination, t);
}

fn update_tileset_image(
    chunk_query: Single<&TilemapChunk>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    let chunk = *chunk_query;
    for event in events.read() {
        if event.is_loaded_with_dependencies(chunk.tileset.id()) {
            let image = images.get_mut(&chunk.tileset).unwrap();
            image
                .reinterpret_stacked_2d_as_array(4)
                .expect("asset should be 2d texture with height evenly divisible by 4");
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

// find the data for an arbitrary tile in the chunk and log its data
fn log_tile(tilemap: Single<(&TilemapChunk, &TilemapChunkTileData)>, mut local: Local<u16>) {
    let (chunk, data) = tilemap.into_inner();
    let Some(tile_data) = data.tile_data_from_tile_pos(chunk.chunk_size, UVec2::new(3, 4)) else {
        return;
    };
    // log when the tile changes
    if tile_data.tileset_index != *local {
        info!(?tile_data, "tile_data changed");
        *local = tile_data.tileset_index;
    }
}
