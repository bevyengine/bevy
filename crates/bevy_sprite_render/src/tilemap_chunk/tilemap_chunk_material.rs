use crate::{AlphaMode2d, Material2d, Material2dPlugin, TileData};
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, embedded_path, Asset, AssetPath, Handle, RenderAssetUsages};
use bevy_color::ColorToPacked;
use bevy_image::{Image, ImageSampler, ToExtents};
use bevy_math::UVec2;
use bevy_reflect::prelude::*;
use bevy_render::render_resource::*;
use bevy_shader::ShaderRef;
use bytemuck::{Pod, Zeroable};

/// Plugin that adds support for tilemap chunk materials.
pub struct TilemapChunkMaterialPlugin;

impl Plugin for TilemapChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "tilemap_chunk_material.wgsl");

        app.add_plugins(Material2dPlugin::<TilemapChunkMaterial>::default());
    }
}

/// Material used for rendering tilemap chunks.
///
/// This material is used internally by the tilemap system to render chunks of tiles
/// efficiently using a single draw call per chunk.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TilemapChunkMaterial {
    pub alpha_mode: AlphaMode2d,

    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub tileset: Handle<Image>,

    #[texture(2, sample_type = "u_int")]
    pub tile_data: Handle<Image>,
}

impl Material2d for TilemapChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("tilemap_chunk_material.wgsl"))
                .with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}

/// Packed per-tile data for use in the `Rgba16Uint` tile data texture in `TilemapChunkMaterial`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PackedTileData {
    tileset_index: u16, // red channel
    color: [u8; 4],     // green and blue channels
    flags: u16,         // alpha channel
}

impl PackedTileData {
    fn empty() -> Self {
        Self {
            tileset_index: u16::MAX,
            color: [0, 0, 0, 0],
            flags: 0,
        }
    }
}

impl From<TileData> for PackedTileData {
    fn from(
        TileData {
            tileset_index,
            color,
            visible,
        }: TileData,
    ) -> Self {
        Self {
            tileset_index,
            color: color.to_srgba().to_u8_array(),
            flags: visible as u16,
        }
    }
}

impl From<Option<TileData>> for PackedTileData {
    fn from(maybe_tile_data: Option<TileData>) -> Self {
        maybe_tile_data
            .map(Into::into)
            .unwrap_or(PackedTileData::empty())
    }
}

pub fn make_chunk_tile_data_image(size: &UVec2, data: &[PackedTileData]) -> Image {
    Image {
        data: Some(bytemuck::cast_slice(data).to_vec()),
        data_order: TextureDataOrder::default(),
        texture_descriptor: TextureDescriptor {
            size: size.to_extents(),
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Uint,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        sampler: ImageSampler::nearest(),
        texture_view_descriptor: None,
        asset_usage: RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        copy_on_resize: false,
    }
}
