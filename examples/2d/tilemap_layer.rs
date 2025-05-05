//! Shows a tilemap layer rendered with a single draw call.

use bevy::{
    prelude::*,
    sprite::{TileData, TileStorage, TilemapChunk, TilemapLayer, Tileset},
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const TILESET_SIZE: u16 = 6;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins.set(ImagePlugin::default_nearest()),))
        .insert_resource(ClearColor(Color::BLACK))
        .add_systems(Startup, setup)
        .add_systems(Update, update_tileset_image)
        .add_systems(Update, update_tilemap_layer)
        .add_observer(on_add_tilemap_chunk)
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct UpdateTimer(Timer);

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let chunk_size = UVec2::splat(32);
    let mut tile_storage = TileStorage::sparse();
    tile_storage.set_chunk_size(chunk_size);

    tile_storage.fill_rect_with(
        IRect::from_corners(IVec2::splat(-32), IVec2::splat(32)),
        |_| {
            let i = rng.gen_range(0..TILESET_SIZE + 1);
            if i == 0 {
                None
            } else {
                Some(TileData {
                    tileset_index: i - 1,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.2),
                    visible: true,
                })
            }
        },
    );

    commands.spawn((
        TilemapLayer::default(),
        Tileset {
            image: assets.load("textures/tileset_array_texture.png"),
            tile_size: UVec2::splat(16),
        },
        tile_storage,
        UpdateTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));

    commands.spawn(Camera2d);
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
            image.reinterpret_stacked_2d_as_array(TILESET_SIZE as u32);
        }
    }
}

fn update_tilemap_layer(time: Res<Time>, mut query: Query<(&mut TileStorage, &mut UpdateTimer)>) {
    for (mut tile_storage, mut timer) in query.iter_mut() {
        timer.tick(time.delta());

        if timer.just_finished() {
            let mut rng = ChaCha8Rng::from_entropy();
            for _ in 0..50 {
                let x = rng.gen_range(-32..32);
                let y = rng.gen_range(-32..32);
                let i = rng.gen_range(0..TILESET_SIZE + 1);
                tile_storage.set(
                    IVec2::new(x, y),
                    if i == 0 {
                        None
                    } else {
                        Some(TileData::from_index(i - 1))
                    },
                );
            }
        }
    }
}

fn on_add_tilemap_chunk(trigger: Trigger<OnAdd, TilemapChunk>, mut commands: Commands) {
    commands
        .entity(trigger.target())
        .insert(ShowAabbGizmo::default());
}
