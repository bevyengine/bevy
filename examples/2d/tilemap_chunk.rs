#![allow(warnings)]

use bevy::{
    prelude::*,
    sprite::{TilemapChunk, TilemapChunkIndices, TilemapChunkMaterial},
};
use bevy_asset::RenderAssetUsages;
use bevy_image::ImageSampler;
use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let chunk_size = UVec2::splat(64);
    let tile_size = UVec2::splat(16);
    let tileset: Handle<Image> = images.add(create_test_tileset(tile_size));
    let indices: Vec<Option<u32>> = (0..chunk_size.x * chunk_size.y)
        .map(|_| rng.gen_range(0..5))
        .map(|i| if i == 0 { None } else { Some(i - 1) })
        .collect();

    commands.spawn((
        TilemapChunk {
            chunk_size,
            tile_size,
            tileset,
        },
        TilemapChunkIndices(indices),
    ));

    commands.spawn(Camera2d);
}

fn create_test_tileset(tile_size: UVec2) -> Image {
    let num_tiles = 4u32;
    let mut data = Vec::new();

    let colors = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
    ];

    for tile_idx in 0..num_tiles {
        let color = colors[tile_idx as usize];
        for _ in 0..tile_size.element_product() {
            data.extend_from_slice(&color);
        }
    }

    Image::new(
        Extent3d {
            width: tile_size.x,
            height: tile_size.y,
            depth_or_array_layers: num_tiles,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
