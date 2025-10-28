//! Shows a tilemap chunk rendered with a single draw call.

use bevy::{
    prelude::*,
    sprite_render::{TileData, TilemapChunk, TilemapChunkTileData, TilingDirection},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            update_tileset_image,
        )
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {

    let chunk_size = UVec2::splat(32);
    let tile_display_size = UVec2::splat(8);
    let tile_data: Vec<Option<TileData>> = (0..chunk_size.element_product())
        .map(|value| Some(TileData {
            tileset_index: 0,
            color: Color::linear_rgb(value as f32 / 32.0 / 32.0 , (value % 32) as f32 / 32.0, 0.0) ,
            .. Default::default()
        }))
        .collect();

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load("textures/array_texture.png"),
            ..default()
        },
        Transform::from_translation(Vec3::new(8.0 * 24.0, -8.0 * 24.0, 0.0)),
        TilemapChunkTileData(tile_data.clone()),
    ));

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load("textures/array_texture.png"),
            tiling_direction: TilingDirection::PosYPosX,
            ..default()
        },
        Transform::from_translation(Vec3::new(8.0 * 24.0, 8.0 * 24.0, 0.0)),
        TilemapChunkTileData(tile_data.clone()),
    ));

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load("textures/array_texture.png"),
            tiling_direction: TilingDirection::NegYNegX,
            ..default()
        },
        Transform::from_translation(Vec3::new(-8.0 * 24.0, -8.0 * 24.0, 0.0)),
        TilemapChunkTileData(tile_data.clone()),
    ));

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load("textures/array_texture.png"),
            tiling_direction: TilingDirection::PosYNegX,
            ..default()
        },
        Transform::from_translation(Vec3::new(-8.0 * 24.0, 8.0 * 24.0, 0.0)),
        TilemapChunkTileData(tile_data.clone()),
    ));

    commands.spawn(Camera2d);
}

fn update_tileset_image(
    chunk_query: Query<&TilemapChunk>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    let chunk = chunk_query.iter().next().unwrap();
    for event in events.read() {
        if event.is_loaded_with_dependencies(chunk.tileset.id()) {
            let image = images.get_mut(&chunk.tileset).unwrap();
            image
                .reinterpret_stacked_2d_as_array(4)
                .expect("asset should be 2d texture with height evenly divisible by 4");
        }
    }
}