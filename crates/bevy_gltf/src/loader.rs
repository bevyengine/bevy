use anyhow::Result;
use bevy_asset::{AssetIoError, AssetLoader, AssetPath, Handle, LoadContext, LoadedAsset};
use bevy_core::Name;
use bevy_ecs::{bevy_utils::BoxedFuture, Entity, World, WorldBuilderSource};
use bevy_math::Mat4;
use bevy_pbr::prelude::{PbrComponents, StandardMaterial};
use bevy_render::{
    mesh::{Indices, Mesh, VertexAttributeValues},
    pipeline::PrimitiveTopology,
    prelude::{Color, Texture},
    texture::{AddressMode, FilterMode, SamplerDescriptor, TextureFormat},
};
use bevy_scene::Scene;
use bevy_skinned_mesh::{MeshSkin, MeshSkinner};
use bevy_transform::{
    hierarchy::{BuildWorldChildren, WorldChildBuilder},
    prelude::{GlobalTransform, Transform},
};
use gltf::{
    mesh::Mode,
    texture::{MagFilter, MinFilter, WrappingMode},
    Primitive,
};
use image::{GenericImageView, ImageFormat};
use std::path::Path;
use thiserror::Error;

/// An error that occurs when loading a GLTF file
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("Unsupported primitive mode.")]
    UnsupportedPrimitive { mode: Mode },
    #[error("Unsupported min filter.")]
    UnsupportedMinFilter { filter: MinFilter },
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

    let mut parents = vec![];
    parents.resize(gltf.nodes().count(), None);
    for node in gltf.nodes() {
        for child in node.children() {
            // Should only happen once, since each node can only have a single parent
            debug_assert!(parents[child.index()].is_none());
            parents[child.index()] = Some(node.index());
        }
    }

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

                if let Some(color_attribute) = reader
                    .read_colors(0)
                    .map(|v| VertexAttributeValues::Uchar4(v.into_rgba_u8().collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_WEIGHT, color_attribute);
                }

                if let Some(weight_attribute) = reader
                    .read_weights(0)
                    .map(|v| VertexAttributeValues::Float4(v.into_f32().collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_WEIGHT, weight_attribute);
                }

                if let Some(joint_attribute) = reader
                    .read_joints(0)
                    .map(|v| VertexAttributeValues::Ushort4(v.into_u16().collect()))
                {
                    mesh.set_attribute(Mesh::ATTRIBUTE_JOINT, joint_attribute);
                }

                load_context.set_labeled_asset(&primitive_label, LoadedAsset::new(mesh));
            };
        }
    }

    for skin in gltf.skins() {
        let skin_label = skin_label(&skin);
        if load_context.has_labeled_asset(&skin_label) {
            panic!("non unique skin labels");
        }

        let reader = skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
        if let Some(inverse_bind_matrices) = reader.read_inverse_bind_matrices() {
            let mut bones_names = vec![];
            let mut bones_parents = vec![];

            for joint in skin.joints() {
                bones_names.push(joint.name().expect("unnamed bone").to_string());
                bones_parents.push(parents[joint.index()]);
            }

            load_context.set_labeled_asset(
                &skin_label,
                LoadedAsset::new(MeshSkin {
                    inverse_bind_matrices: inverse_bind_matrices
                        .map(|x| Mat4::from_cols_array_2d(&x))
                        .collect(),
                    bones_names,
                    bones_parents,
                }),
            );
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
                    sampler: texture_sampler(&texture)?,
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

    // Each node will be mapped to a slot inside this `entity_lookup`
    let mut entity_lookup = vec![];
    entity_lookup.resize_with(gltf.nodes().count(), || None);

    for scene in gltf.scenes() {
        world_builder.spawn((Transform::default(), GlobalTransform::default()));

        if let Some(name) = scene.name() {
            world_builder.with(Name(name.to_string()));
        }

        world_builder.with_children(|parent| {
            for node in scene.nodes() {
                load_node(&node, &node, parent, load_context, &mut entity_lookup, 0);
            }
        });
    }

    load_context.set_default_asset(LoadedAsset::new(Scene::new(world)));

    Ok(())
}

fn load_node(
    root_node: &gltf::Node,
    node: &gltf::Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    entity_lookup: &mut Vec<Option<Entity>>,
    mut depth: usize, // Used to debug (uncomment the prints)
) {
    // print!("{}", "    ".repeat(depth));
    // println!("-{:?}", node.name());
    depth += 1;

    // NOTE: Avoid heavy computations when possible, also the blender exporter likes to
    // output decomposed matrixes with is better for us
    let (translation, rotation, scale) = node.transform().decomposed();
    world_builder.spawn((
        Transform {
            translation: translation.into(),
            rotation: rotation.into(),
            scale: scale.into(),
        },
        GlobalTransform::default(),
    ));

    let node_entity = world_builder
        .current_entity()
        .expect("entity wasn't created");
    entity_lookup[node.index()] = Some(node_entity);

    if let Some(name) = node.name() {
        world_builder.with(Name(name.to_string()));
    }

    if let Some(skin) = node.skin() {
        let skin_label = skin_label(&skin);
        let skin_asset_path = AssetPath::new_ref(load_context.path(), Some(&skin_label));
        let skin_handle: Handle<MeshSkin> = load_context.get_handle(skin_asset_path);
        world_builder.with(skin_handle);

        // TODO: Mesh skinner needs a at least a reference to the skeleton root for it to work
        // for this reason it need to keep track of all entities created by their index in the gltf node vec
        let skeleton_root = skin.skeleton().map_or(root_node.index(), |n| n.index());
        world_builder.with(MeshSkinner::with_skeleton(
            entity_lookup[skeleton_root].expect("missing skeleton root entity"),
        ));
    }

    world_builder.with_children(|parent| {
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                let primitive_label = primitive_label(&mesh, &primitive);
                let mesh_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                let material = primitive.material();
                let material_label = material_label(&material);
                let material_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&material_label));

                // print!("{}", "    ".repeat(depth));
                // println!("-Mesh_{:?}_{}", mesh.name(), primitive.index());

                parent.spawn(PbrComponents {
                    mesh: load_context.get_handle(mesh_asset_path),
                    material: load_context.get_handle(material_asset_path),
                    ..Default::default()
                });
            }
        }

        for child in node.children() {
            load_node(
                root_node,
                &child,
                parent,
                load_context,
                entity_lookup,
                depth,
            );
        }
    });
}

fn skin_label(skin: &gltf::Skin) -> String {
    format!("Skin{}", skin.index())
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
