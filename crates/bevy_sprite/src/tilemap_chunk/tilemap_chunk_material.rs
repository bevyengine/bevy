use crate::{Material2d, Material2dPlugin};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Asset, Handle};
use bevy_image::Image;
use bevy_reflect::prelude::*;
use bevy_render::render_resource::*;

pub const TILEMAP_CHUNK_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("40f33e62-82f8-4578-b3fa-f22989e7c4bb");

/// Plugin that adds support for tilemap chunk materials.
#[derive(Default)]
pub struct TilemapChunkMaterialPlugin;

impl Plugin for TilemapChunkMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TILEMAP_CHUNK_MATERIAL_SHADER_HANDLE,
            "tilemap_chunk_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(Material2dPlugin::<TilemapChunkMaterial>::default());
    }
}

/// Material used for rendering tilemap chunks.
///
/// This material is used internally by the tilemap system to render chunks of tiles
/// efficiently using a single draw call per chunk.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TilemapChunkMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub tileset: Handle<Image>,

    #[texture(2, sample_type = "u_int")]
    pub indices: Handle<Image>,
}

impl Material2d for TilemapChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        TILEMAP_CHUNK_MATERIAL_SHADER_HANDLE.into()
    }
}
