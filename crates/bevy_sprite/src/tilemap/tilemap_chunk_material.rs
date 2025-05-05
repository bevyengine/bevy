use crate::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Asset, Handle};
use bevy_image::Image;
use bevy_reflect::prelude::*;
use bevy_render::{
    mesh::{Mesh, MeshVertexAttribute, MeshVertexBufferLayoutRef},
    render_resource::*,
};

pub const TILEMAP_CHUNK_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("40f33e62-82f8-4578-b3fa-f22989e7c4bb");

pub const ATTRIBUTE_TILE_INDEX: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileIndex", 264043692, VertexFormat::Uint32);

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
    pub alpha_mode: AlphaMode2d,

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

    fn vertex_shader() -> ShaderRef {
        TILEMAP_CHUNK_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
            ATTRIBUTE_TILE_INDEX.at_shader_location(5),
        ])?;
        descriptor.vertex.buffers = vec![vertex_layout];
        Ok(())
    }
}
