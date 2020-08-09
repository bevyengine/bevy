use bevy_render::{
    mesh::{Mesh, VertexAttribute, VertexAttributeValues},
    pipeline::PrimitiveTopology,
};

use anyhow::Result;
use bevy_asset::AssetLoader;
use gltf::{buffer::Source, iter, mesh::Mode};
use std::{fs, io, path::Path};
use thiserror::Error;

/// Loads meshes from GLTF files into Mesh assets
///
/// NOTE: eventually this will loading into Scenes instead of Meshes
#[derive(Default)]
pub struct GltfLoader;

impl AssetLoader<Mesh> for GltfLoader {
    fn from_bytes(&self, asset_path: &Path, bytes: Vec<u8>) -> Result<Mesh> {
        let mesh = load_gltf(asset_path, bytes)?;
        Ok(mesh)
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["gltf"];
        EXTENSIONS
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
    #[error("Binary buffers not supported yet.")]
    BinaryBuffersUnsupported,
}

fn get_primitive_topology(mode: Mode) -> Result<PrimitiveTopology, GltfError> {
    match mode {
        Mode::Points => Ok(PrimitiveTopology::PointList),
        Mode::Lines => Ok(PrimitiveTopology::LineList),
        Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
        Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
        Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
        mode @ _ => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}

// TODO: this should return a scene
pub fn load_gltf(asset_path: &Path, bytes: Vec<u8>) -> Result<Mesh, GltfError> {
    let gltf = gltf::Gltf::from_slice(&bytes)?;
    let buffer_data = load_buffers(gltf.buffers(), asset_path)?;
    for scene in gltf.scenes() {
        for node in scene.nodes() {
            return Ok(load_node(&buffer_data, &node, 1)?);
        }
    }

    // TODO: remove this when full gltf support is added
    panic!("no mesh found!")
}

fn load_node(buffer_data: &[Vec<u8>], node: &gltf::Node, depth: i32) -> Result<Mesh, GltfError> {
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let primitive_topology = get_primitive_topology(primitive.mode())?;
            let mut mesh = Mesh::new(primitive_topology);
            reader
                .read_positions()
                .map(|v| VertexAttribute {
                    name: "Vertex_Position".into(),
                    values: VertexAttributeValues::Float3(v.collect()),
                })
                .map(|vertex_attribute| mesh.attributes.push(vertex_attribute));

            reader
                .read_normals()
                .map(|v| VertexAttribute {
                    name: "Vertex_Normal".into(),
                    values: VertexAttributeValues::Float3(v.collect()),
                })
                .map(|vertex_attribute| mesh.attributes.push(vertex_attribute));

            reader
                .read_tex_coords(0)
                .map(|v| VertexAttribute {
                    name: "Vertex_Uv".into(),
                    values: VertexAttributeValues::Float2(v.into_f32().collect()),
                })
                .map(|vertex_attribute| mesh.attributes.push(vertex_attribute));

            reader.read_indices().map(|indices| {
                mesh.indices = Some(indices.into_u32().collect::<Vec<u32>>());
            });

            return Ok(mesh);
        }
    }

    for child in node.children() {
        return Ok(load_node(buffer_data, &child, depth + 1)?);
    }

    panic!("failed to find mesh")
}

fn load_buffers(buffers: iter::Buffers, asset_path: &Path) -> Result<Vec<Vec<u8>>, GltfError> {
    let mut buffer_data = Vec::new();
    for buffer in buffers {
        match buffer.source() {
            Source::Uri(uri) => {
                if uri.starts_with("data:") {
                } else {
                    let buffer_path = asset_path.parent().unwrap().join(uri);
                    let buffer_bytes = fs::read(buffer_path)?;
                    buffer_data.push(buffer_bytes);
                }
            }
            Source::Bin => return Err(GltfError::BinaryBuffersUnsupported),
        }
    }

    Ok(buffer_data)
}
