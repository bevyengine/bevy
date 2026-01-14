//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    color::palettes::tailwind::RED_400,
    image::{ImageArrayLayout, ImageLoaderSettings},
    prelude::*,
    sprite::TileStorage,
    sprite_render::{TileRenderData, TilemapChunkRenderData},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (setup, spawn_fake_player).chain())
        .add_systems(Update, (update_tilemap, move_player, log_tile))
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
    let tile_data: Vec<Option<TileRenderData>> = (0..chunk_size.element_product())
        .map(|_| rng.random_range(0..5))
        .map(|i| {
            if i == 0 {
                None
            } else {
                Some(TileRenderData::from_tileset_index(i - 1))
            }
        })
        .collect();

    let mut storage = TileStorage::<TileRenderData>::new(chunk_size);
    storage.tiles = tile_data;
    commands.spawn((
        TilemapChunkRenderData {
            chunk_size,
            tile_display_size,
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
        storage,
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
    chunk: Single<&TilemapChunkRenderData>,
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
    chunk: Single<&TilemapChunkRenderData>,
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

fn update_tilemap(
    time: Res<Time>,
    mut query: Query<(&mut TileStorage<TileRenderData>, &mut UpdateTimer)>,
    mut rng: ResMut<SeededRng>,
) {
    for (mut tile_data, mut timer) in query.iter_mut() {
        timer.tick(time.delta());

        if timer.just_finished() {
            for _ in 0..50 {
                let index = rng.random_range(0..tile_data.tiles.len());
                tile_data.tiles[index] =
                    Some(TileRenderData::from_tileset_index(rng.random_range(0..5)));
            }
        }
    }
}

// find the data for an arbitrary tile in the chunk and log its data
fn log_tile(
    tilemap: Single<(&TilemapChunkRenderData, &TileStorage<TileRenderData>)>,
    mut local: Local<u16>,
) {
    let (_, data) = tilemap.into_inner();
    let Some(tile_data) = data.get_at(UVec2::new(3, 4)) else {
        return;
    };
    // log when the tile changes
    if tile_data.tileset_index != *local {
        info!(?tile_data, "tile_data changed");
        *local = tile_data.tileset_index;
    }
}
