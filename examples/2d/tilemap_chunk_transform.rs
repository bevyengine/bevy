//! Shows a tilemap chunk rendered with a single draw call, including transforming tiles (rotate, mirror) and using different tileset indices, colors, alpha and visibility to show all tile features.

use bevy::{
    image::{ImageArrayLayout, ImageLoaderSettings},
    prelude::*,
    sprite_render::{AlphaMode2d, TileData, TileTransform, TilemapChunk, TilemapChunkTileData},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(ClearColor(Color::srgb(0.5, 0.5, 0.9)))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let chunk_size = UVec2::splat(8);
    let tile_display_size = UVec2::splat(64);

    // We'll use each possible transform, one per column
    let transforms = [
        TileTransform::None,
        TileTransform::Rotate90,
        TileTransform::Rotate180,
        TileTransform::Rotate270,
        TileTransform::MirrorX,
        TileTransform::MirrorXRotate90,
        TileTransform::MirrorXRotate180,
        TileTransform::MirrorXRotate270,
    ];

    // Show different color/alpha on each row
    let colors = [
        Color::WHITE,
        Color::linear_rgb(1.0, 0.0, 0.0),
        Color::linear_rgb(0.0, 1.0, 0.0),
        Color::linear_rgb(0.0, 0.0, 1.0),
        Color::linear_rgba(1.0, 0.0, 0.0, 0.25),
        Color::linear_rgba(0.0, 1.0, 0.0, 0.25),
        Color::linear_rgba(0.0, 0.0, 1.0, 0.25),
        Color::linear_rgba(1.0, 1.0, 1.0, 0.5),
    ];

    let tile_data = (0..chunk_size.element_product())
        .map(|i| {
            let row = i / 8;
            let col = i % 8;
            Some(TileData {
                // Alternate tiles per row
                tileset_index: (row % 2) as u16,
                color: colors[row as usize],
                // Last (top) row is invisible
                visible: row != 7,
                transform: transforms[col as usize],
            })
        })
        .collect();

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset: assets.load_with_settings(
                "textures/arrow.png",
                |settings: &mut ImageLoaderSettings| {
                    // The tileset texture is expected to be an array of tile textures, so we tell the
                    // `ImageLoader` that our texture is composed of 2 stacked tile images.
                    settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 2 });
                },
            ),
            alpha_mode: AlphaMode2d::Blend,
        },
        TilemapChunkTileData(tile_data),
    ));

    commands.spawn(Camera2d);
}
