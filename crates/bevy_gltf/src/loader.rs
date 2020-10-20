use anyhow::Result;
use bevy_asset::{AssetIoError, AssetLoader, AssetPath, LoadContext, LoadedAsset};
use bevy_ecs::{bevy_utils::BoxedFuture, World, WorldBuilderSource};
use bevy_math::Mat4;
use bevy_pbr::prelude::{PbrComponents, StandardMaterial};
use bevy_render::{
    mesh::{Indices, Mesh, VertexAttribute},
    pipeline::PrimitiveTopology,
    prelude::{Color, Texture},
    texture::TextureFormat,
};
use bevy_scene::Scene;
use bevy_transform::{
    hierarchy::{BuildWorldChildren, WorldChildBuilder},
    prelude::{GlobalTransform, Transform},
};
use gltf::{mesh::Mode, Primitive};
use image::{GenericImageView, ImageFormat};
use std::path::Path;
use thiserror::Error;

/// An error that occurs when loading a GLTF file
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("Unsupported primitive mode.")]
    UnsupportedPrimitive { mode: Mode },
    #[error("Invalid GLTF file.")]
    Gltf(#[from] gltf::Error),
    #[error("Binary blob is missing.")]
    MissingBlob,
    #[error("Failed to decode base64 mesh data.")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("Unsupported buffer format.")]
    BufferFormatUnsupported,
    #[error("Invalid image mime type.")]
    InvalidImageMimeType(String),
    #[error("Failed to convert image to rgb8.")]
    ImageRgb8ConversionFailure,
    #[error("Failed to load an image.")]
    ImageError(#[from] image::ImageError),
    #[error("Failed to load an asset path.")]
    AssetIoError(#[from] AssetIoError),
}

/// Loads meshes from GLTF files into Mesh assets
#[derive(Default)]
pub struct GltfLoader;

impl AssetLoader for GltfLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move { Ok(load_gltf(bytes, load_context).await?) })
    }

    fn extensions(&self) -> &[&str] {
        static EXTENSIONS: &[&str] = &["gltf", "glb"];
        EXTENSIONS
    }
}

async fn load_gltf<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
) -> Result<(), GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let mut world = World::default();
    let buffer_data = load_buffers(&gltf, load_context, load_context.path()).await?;

    let world_builder = &mut world.build();

    for mesh in gltf.meshes() {
        for primitive in mesh.primitives() {
            let primitive_label = primitive_label(&mesh, &primitive);
            if !load_context.has_labeled_asset(&primitive_label) {
                let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let primitive_topology = get_primitive_topology(primitive.mode())?;

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
                    mesh.indices = Some(Indices::U32(indices.into_u32().collect()));
                };

                load_context.set_labeled_asset(&primitive_label, LoadedAsset::new(mesh));
            };
        }
    }

    for texture in gltf.textures() {
        if let gltf::image::Source::View { view, mime_type } = texture.source().source() {
            let start = view.offset() as usize;
            let end = (view.offset() + view.length()) as usize;
            let buffer = &buffer_data[view.buffer().index()][start..end];
            let format = match mime_type {
                "image/png" => Ok(ImageFormat::Png),
                "image/jpeg" => Ok(ImageFormat::Jpeg),
                _ => Err(GltfError::InvalidImageMimeType(mime_type.to_string())),
            }?;
            let image = image::load_from_memory_with_format(buffer, format)?;
            let size = image.dimensions();
            let image = image
                .as_rgba8()
                .ok_or(GltfError::ImageRgb8ConversionFailure)?;

            let texture_label = texture_label(&texture);
            load_context.set_labeled_asset(
                &texture_label,
                LoadedAsset::new(Texture {
                    data: image.clone().into_vec(),
                    size: bevy_math::f32::vec2(size.0 as f32, size.1 as f32),
                    format: TextureFormat::Rgba8Unorm,
                }),
            );
        }
    }

    for material in gltf.materials() {
        let material_label = material_label(&material);
        let pbr = material.pbr_metallic_roughness();
        let mut dependencies = Vec::new();
        let texture_handle = if let Some(info) = pbr.base_color_texture() {
            match info.texture().source().source() {
                gltf::image::Source::View { .. } => {
                    let label = texture_label(&info.texture());
                    let path = AssetPath::new_ref(load_context.path(), Some(&label));
                    Some(load_context.get_handle(path))
                }
                gltf::image::Source::Uri { uri, .. } => {
                    let parent = load_context.path().parent().unwrap();
                    let image_path = parent.join(uri);
                    let asset_path = AssetPath::new(image_path, None);
                    let handle = load_context.get_handle(asset_path.clone());
                    dependencies.push(asset_path);
                    Some(handle)
                }
            }
        } else {
            None
        };
        let color = pbr.base_color_factor();
        load_context.set_labeled_asset(
            &material_label,
            LoadedAsset::new(StandardMaterial {
                albedo: Color::rgba(color[0], color[1], color[2], color[3]),
                albedo_texture: texture_handle,
                ..Default::default()
            })
            .with_dependencies(dependencies),
        )
    }

    for scene in gltf.scenes() {
        let mut err = None;
        world_builder
            .spawn((Transform::default(), GlobalTransform::default()))
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = load_node(&node, parent, load_context, &buffer_data);
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            });
        if let Some(Err(err)) = err {
            return Err(err);
        }
    }

    load_context.set_default_asset(LoadedAsset::new(Scene::new(world)));

    Ok(())
}

fn load_node(
    node: &gltf::Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    buffer_data: &[Vec<u8>],
) -> Result<(), GltfError> {
    let transform = node.transform();
    let mut gltf_error = None;
    world_builder
        .spawn((
            Transform::from_matrix(Mat4::from_cols_array_2d(&transform.matrix())),
            GlobalTransform::default(),
        ))
        .with_children(|parent| {
            if let Some(mesh) = node.mesh() {
                for primitive in mesh.primitives() {
                    let primitive_label = primitive_label(&mesh, &primitive);
                    let mesh_asset_path =
                        AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                    let material = primitive.material();
                    let material_label = material_label(&material);
                    let material_asset_path =
                        AssetPath::new_ref(load_context.path(), Some(&material_label));
                    parent.spawn(PbrComponents {
                        mesh: load_context.get_handle(mesh_asset_path),
                        material: load_context.get_handle(material_asset_path),
                        ..Default::default()
                    });
                }
            }

            if parent.current_entity().is_none() {
                return;
            }

            parent.with_children(|parent| {
                for child in node.children() {
                    if let Err(err) = load_node(&child, parent, load_context, buffer_data) {
                        gltf_error = Some(err);
                        return;
                    }
                }
            });
        });
    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

fn primitive_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!("Mesh{}/Primitive{}", mesh.index(), primitive.index())
}

fn material_label(material: &gltf::Material) -> String {
    if let Some(index) = material.index() {
        format!("Material{}", index)
    } else {
        "MaterialDefault".to_string()
    }
}

fn texture_label(texture: &gltf::Texture) -> String {
    format!("Texture{}", texture.index())
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

async fn load_buffers(
    gltf: &gltf::Gltf,
    load_context: &LoadContext<'_>,
    asset_path: &Path,
) -> Result<Vec<Vec<u8>>, GltfError> {
    const OCTET_STREAM_URI: &str = "data:application/octet-stream;base64,";

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                if uri.starts_with("data:") {
                    if uri.starts_with(OCTET_STREAM_URI) {
                        buffer_data.push(base64::decode(&uri[OCTET_STREAM_URI.len()..])?);
                    } else {
                        return Err(GltfError::BufferFormatUnsupported);
                    }
                } else {
                    // TODO: Remove this and add dep
                    let buffer_path = asset_path.parent().unwrap().join(uri);
                    let buffer_bytes = load_context.read_asset_bytes(buffer_path).await?;
                    buffer_data.push(buffer_bytes);
                }
            }
            gltf::buffer::Source::Bin => {
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
