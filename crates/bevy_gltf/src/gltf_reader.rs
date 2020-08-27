// Helper methods for the reading of GLTF files

use std::{fs, io, path::Path};

use anyhow::Result;
use gltf::{buffer::Source, mesh::Mode};
use image::{GenericImageView, ImageFormat};
use log::warn;
use thiserror::Error;

use bevy_render::{
    mesh::{Mesh, VertexAttribute},
    pipeline::PrimitiveTopology,
    texture::{Texture, TextureFormat},
};

/// The result of loading a GLTF file.
/// May be possible to extend to support multiple meshes & materials.
pub struct GltfAssets {
    pub mesh: Vec<Mesh>,
    pub texture: Vec<Texture>,
}

impl Default for GltfAssets {
    fn default() -> Self {
        GltfAssets {
            mesh: Vec::new(),
            texture: Vec::new(),
        }
    }
}

/// An error that occurs when loading a GLTF file
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("Unsupported primitive mode.")]
    UnsupportedPrimitive { mode: Mode },
    #[error("Invalid GLTF file.")]
    Gltf(#[from] gltf::Error),
    #[error("Failed to load file.")]
    Io(#[from] io::Error),
    #[error("Binary blob is missing.")]
    MissingBlob,
    #[error("Failed to decode base64 mesh data.")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Unsupported buffer format.")]
    BufferFormatUnsupported,
}

fn get_primitive_topology(mode: Mode) -> Result<PrimitiveTopology, GltfError> {
    match mode {
        Mode::Points => Ok(PrimitiveTopology::PointList),
        Mode::Lines => Ok(PrimitiveTopology::LineList),
        Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
        Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
        Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
        mode => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}

// TODO: this should return a scene
pub fn load_gltf(asset_path: &Path, bytes: Vec<u8>) -> anyhow::Result<GltfAssets> {
    let gltf = gltf::Gltf::from_slice(&bytes)?;
    let mut assets = GltfAssets::default();
    let buffer_data = load_buffers(&gltf, asset_path)?;
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            if let Err(e) = load_node(&mut assets, &buffer_data, &node, 1) {
                warn!(
                    "While loading GLTF {} encountered problem {:?}",
                    asset_path.display(),
                    e
                );
            }
        }
    }

    return Ok(assets);
}

/// Parse the node. Set the data in the passed-in GltfAssets if appropriate.
fn load_node(
    asset: &mut GltfAssets,
    buffer_data: &[Vec<u8>],
    node: &gltf::Node,
    depth: i32,
) -> anyhow::Result<()> {
    if let Some(mesh) = node.mesh() {
        // Read Mesh info from primitives into Bevy's formats
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let primitive_topology = match get_primitive_topology(primitive.mode()) {
                Ok(p) => p,
                Err(e) => {
                    warn!("While loading GLTF, encountered problem: {:?}", e);
                    continue;
                }
            };

            let mut mesh = Mesh::new(primitive_topology);

            if let Some(vertex_attribute) = reader
                .read_positions()
                .map(|v| VertexAttribute::position(v.collect()))
            {
                mesh.attributes.push(vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_normals()
                .map(|v| VertexAttribute::normal(v.collect()))
            {
                mesh.attributes.push(vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_tex_coords(0)
                .map(|v| VertexAttribute::uv(v.into_f32().collect()))
            {
                mesh.attributes.push(vertex_attribute);
            }

            if let Some(indices) = reader.read_indices() {
                mesh.indices = Some(indices.into_u32().collect::<Vec<u32>>());
            };

            asset.mesh.push(mesh);

            // Read the material info and apply it
            let material = primitive.material();
            let texture = material.pbr_metallic_roughness().base_color_texture();
            if texture.is_some() {
                let image = texture.unwrap().texture().source().source();
                match image {
                    gltf::image::Source::Uri {
                        uri: _uri,
                        mime_type: _mime_type,
                    } => panic!("URIs are not supported by the GLTF loader"),
                    gltf::image::Source::View { view, mime_type } => {
                        let start = view.offset() as usize;
                        let end = (view.offset() + view.length()) as usize;
                        let buffer = &buffer_data[view.buffer().index()][start..end];
                        let format = match mime_type {
                            "image/png" => ImageFormat::Png,
                            "image/jpeg" => ImageFormat::Jpeg,
                            _ => {
                                warn!("While loading GLTF and converting image, could not identify image type '{}'", mime_type);
                                continue;
                            }
                        };
                        let image = match image::load_from_memory_with_format(buffer, format) {
                            Ok(image) => image,
                            Err(e) => {
                                warn!("While loading GLTF and converting image, encountered problem: {:?}", e.to_string());
                                continue;
                            }
                        };
                        let size = image.dimensions();
                        let image = image.as_rgba8().ok_or_else(|| {
                            warn!("While loading GLTF, converting image to rgba8 failed.");
                            anyhow::anyhow!("Could not convert image to rgba8")
                        })?;

                        let texture = Texture {
                            data: image.clone().into_vec(),
                            size: bevy_math::f32::vec2(size.0 as f32, size.1 as f32),
                            format: TextureFormat::Rgba8Unorm,
                        };

                        asset.texture.push(texture);
                    }
                }
            }
        }
    }

    for child in node.children() {
        load_node(asset, buffer_data, &child, depth + 1)?;
    }

    Ok(())
}

fn load_buffers(gltf: &gltf::Gltf, asset_path: &Path) -> Result<Vec<Vec<u8>>, GltfError> {
    const OCTET_STREAM_URI: &str = "data:application/octet-stream;base64,";

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            Source::Uri(uri) => {
                if uri.starts_with("data:") {
                    if uri.starts_with(OCTET_STREAM_URI) {
                        buffer_data.push(base64::decode(&uri[OCTET_STREAM_URI.len()..])?);
                    } else {
                        return Err(GltfError::BufferFormatUnsupported);
                    }
                } else {
                    let buffer_path = asset_path.parent().unwrap().join(uri);
                    let buffer_bytes = fs::read(buffer_path)?;
                    buffer_data.push(buffer_bytes);
                }
            }
            Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob);
                }
            }
        }
    }

    Ok(buffer_data)
}
