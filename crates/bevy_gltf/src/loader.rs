use anyhow::Result;
use bevy_asset::{AssetIoError, AssetLoader, AssetPath, LoadContext, LoadedAsset};
use bevy_ecs::{bevy_utils::BoxedFuture, World, WorldBuilderSource};
use bevy_math::Mat4;
use bevy_pbr::prelude::{PbrBundle, StandardMaterial};
use bevy_render::{
    camera::{
        Camera, CameraProjection, OrthographicProjection, PerspectiveProjection, VisibleEntities,
    },
    mesh::{Indices, Mesh, VertexAttributeValues},
    pipeline::PrimitiveTopology,
    prelude::{Color, Texture},
    render_graph::base,
    texture::{
        AddressMode, Extent3d, FilterMode, SamplerDescriptor, TextureDimension, TextureFormat,
    },
};
use bevy_scene::Scene;
use bevy_transform::{
    hierarchy::{BuildWorldChildren, WorldChildBuilder},
    prelude::{GlobalTransform, Transform},
};
use gltf::{
    mesh::Mode,
    texture::{MagFilter, MinFilter, WrappingMode},
    Material, Primitive,
};
use image::{GenericImageView, ImageFormat};
use std::path::Path;
use thiserror::Error;

/// An error that occurs when loading a GLTF file
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive { mode: Mode },
    #[error("unsupported min filter")]
    UnsupportedMinFilter { filter: MinFilter },
    #[error("invalid GLTF file")]
    Gltf(#[from] gltf::Error),
    #[error("binary blob is missing")]
    MissingBlob,
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    #[error("invalid image mime type")]
    InvalidImageMimeType(String),
    #[error("failed to load an image")]
    ImageError(#[from] image::ImageError),
    #[error("failed to load an asset path")]
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
        &["gltf", "glb"]
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
                    .map(|v| VertexAttributeValues::Float3(v.collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertex_attribute);
                }

                if let Some(vertex_attribute) = reader
                    .read_normals()
                    .map(|v| VertexAttributeValues::Float3(v.collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vertex_attribute);
                }

                if let Some(vertex_attribute) = reader
                    .read_tex_coords(0)
                    .map(|v| VertexAttributeValues::Float2(v.into_f32().collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vertex_attribute);
                }

                if let Some(indices) = reader.read_indices() {
                    mesh.set_indices(Some(Indices::U32(indices.into_u32().collect())));
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
            let image = image.into_rgba8();

            let texture_label = texture_label(&texture);
            load_context.set_labeled_asset(
                &texture_label,
                LoadedAsset::new(Texture {
                    data: image.clone().into_vec(),
                    size: Extent3d::new(size.0, size.1, 1),
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba8Unorm,
                    sampler: texture_sampler(&texture)?,
                }),
            );
        }
    }

    for material in gltf.materials() {
        load_material(&material, load_context);
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

fn load_material(material: &Material, load_context: &mut LoadContext) {
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

fn load_node(
    gltf_node: &gltf::Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    buffer_data: &[Vec<u8>],
) -> Result<(), GltfError> {
    let transform = gltf_node.transform();
    let mut gltf_error = None;
    let node = world_builder.spawn((
        Transform::from_matrix(Mat4::from_cols_array_2d(&transform.matrix())),
        GlobalTransform::default(),
    ));

    // create camera node
    if let Some(camera) = gltf_node.camera() {
        node.with(VisibleEntities {
            ..Default::default()
        });

        match camera.projection() {
            gltf::camera::Projection::Orthographic(orthographic) => {
                let xmag = orthographic.xmag();
                let ymag = orthographic.ymag();
                let orthographic_projection: OrthographicProjection = OrthographicProjection {
                    left: -xmag,
                    right: xmag,
                    top: ymag,
                    bottom: -ymag,
                    far: orthographic.zfar(),
                    near: orthographic.znear(),
                    ..Default::default()
                };

                node.with(Camera {
                    name: Some(base::camera::CAMERA_2D.to_owned()),
                    projection_matrix: orthographic_projection.get_projection_matrix(),
                    ..Default::default()
                });
                node.with(orthographic_projection);
            }
            gltf::camera::Projection::Perspective(perspective) => {
                let mut perspective_projection: PerspectiveProjection = PerspectiveProjection {
                    fov: perspective.yfov(),
                    near: perspective.znear(),
                    ..Default::default()
                };
                if let Some(zfar) = perspective.zfar() {
                    perspective_projection.far = zfar;
                }
                if let Some(aspect_ratio) = perspective.aspect_ratio() {
                    perspective_projection.aspect_ratio = aspect_ratio;
                }
                node.with(Camera {
                    name: Some(base::camera::CAMERA_3D.to_owned()),
                    projection_matrix: perspective_projection.get_projection_matrix(),
                    ..Default::default()
                });
                node.with(perspective_projection);
            }
        }
    }

    node.with_children(|parent| {
        if let Some(mesh) = gltf_node.mesh() {
            // append primitives
            for primitive in mesh.primitives() {
                let material = primitive.material();
                let material_label = material_label(&material);

                // This will make sure we load the default material now since it would not have been
                // added when iterating over all the gltf materials (since the default material is
                // not explicitly listed in the gltf).
                if !load_context.has_labeled_asset(&material_label) {
                    load_material(&material, load_context);
                }

                let primitive_label = primitive_label(&mesh, &primitive);
                let mesh_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                let material_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&material_label));

                parent.spawn(PbrBundle {
                    mesh: load_context.get_handle(mesh_asset_path),
                    material: load_context.get_handle(material_asset_path),
                    ..Default::default()
                });
            }
        }

        // append other nodes
        for child in gltf_node.children() {
            if let Err(err) = load_node(&child, parent, load_context, buffer_data) {
                gltf_error = Some(err);
                return;
            }
        }
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

fn texture_sampler(texture: &gltf::Texture) -> Result<SamplerDescriptor, GltfError> {
    let gltf_sampler = texture.sampler();

    Ok(SamplerDescriptor {
        address_mode_u: texture_address_mode(&gltf_sampler.wrap_s()),
        address_mode_v: texture_address_mode(&gltf_sampler.wrap_t()),

        mag_filter: gltf_sampler
            .mag_filter()
            .map(|mf| match mf {
                MagFilter::Nearest => FilterMode::Nearest,
                MagFilter::Linear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().mag_filter),

        min_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest => Ok(FilterMode::Nearest),
                MinFilter::Linear => Ok(FilterMode::Linear),
                filter => Err(GltfError::UnsupportedMinFilter { filter }),
            })
            .transpose()?
            .unwrap_or(SamplerDescriptor::default().min_filter),

        ..Default::default()
    })
}

fn texture_address_mode(gltf_address_mode: &gltf::texture::WrappingMode) -> AddressMode {
    match gltf_address_mode {
        WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
        WrappingMode::Repeat => AddressMode::Repeat,
        WrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
    }
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
