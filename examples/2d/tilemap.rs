//! Shows a tilemap layer where each tile is an entity.

use bevy::{
    color::palettes::tailwind,
    dev_tools::fps_overlay::FpsOverlayPlugin,
    prelude::*,
    sprite::{TileIndex, TileOf, TilePosition, TileStorage, TilemapChunk, TilemapLayer, Tileset},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const NUM_TILES: u16 = 6;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(FpsOverlayPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (update_tileset_image, update_tilemap))
        .add_observer(on_tilemap_chunk_inserted)
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

    let map_size = UVec2::splat(10);

    commands
        .spawn((
            TilemapLayer {
                // chunk_size: UVec2::splat(2),
                ..default()
            },
            TileStorage::new(map_size),
            Tileset {
                image: assets.load("textures/tileset_array_texture.png"),
                tile_size: UVec2::splat(8),
            },
            UpdateTimer(Timer::from_seconds(0.5, TimerMode::Repeating)),
        ))
        .with_related_entities::<TileOf>(|t| {
            t.spawn((
                TilePosition(uvec2(0, 0)),
                TileIndex(0),
                // TileIndex(rng.gen_range(0..NUM_TILES)),
            ));
            // for x in 0..map_size.x {
            //     for y in 0..map_size.y {
            //         t.spawn((
            //             Visibility::default(),
            //             Transform::from_xyz(x as f32, y as f32, 0.0),
            //             TilePosition(uvec2(x, y)),
            //             TileIndex(rng.gen_range(0..NUM_TILES)),
            //         ));
            //     }
            // }
        });

    commands.spawn(Camera2d);

    commands.insert_resource(SeededRng(rng));
}

fn update_tileset_image(
    tileset_query: Single<&Tileset>,
    mut events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    let tileset = *tileset_query;
    for event in events.read() {
        if event.is_loaded_with_dependencies(tileset.image.id()) {
            let image = images.get_mut(&tileset.image).unwrap();
            image.reinterpret_stacked_2d_as_array(NUM_TILES as u32);
        }
    }
}

fn update_tilemap(
    time: Res<Time>,
    mut tilemap_layer_query: Query<(&TileStorage, &mut UpdateTimer)>,
    mut tile_query: Query<&mut TilePosition>,
    mut rng: ResMut<SeededRng>,
) {
    for (tile_storage, mut timer) in &mut tilemap_layer_query {
        timer.tick(time.delta());
        if timer.just_finished() {
            let size = tile_storage.size();
            for mut tile_pos in &mut tile_query {
                tile_pos.0 = uvec2(rng.gen_range(0..size.x), rng.gen_range(0..size.y));
            }
        }
    }
}

fn on_tilemap_chunk_inserted(trigger: On<Insert, TilemapChunk>, mut commands: Commands) {
    commands.entity(trigger.target()).insert(ShowAabbGizmo {
        color: Some(tailwind::RED_300.into()),
    });
}
