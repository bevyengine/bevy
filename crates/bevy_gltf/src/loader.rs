use anyhow::Result;
use bevy_asset::{
    AssetIoError, AssetLoader, AssetPath, BoxedFuture, Handle, LoadContext, LoadedAsset,
};
use bevy_core::Name;
use bevy_ecs::world::World;
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
    texture::{AddressMode, FilterMode, ImageType, SamplerDescriptor, TextureError, TextureFormat},
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
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use thiserror::Error;

use crate::{Gltf, GltfNode};

/// An error that occurs when loading a GLTF file
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive { mode: Mode },
    #[error("invalid GLTF file: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("binary blob is missing")]
    MissingBlob,
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    #[error("{0}")]
    ImageError(#[from] TextureError),
    #[error("failed to load an asset path: {0}")]
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
    let buffer_data = load_buffers(&gltf, load_context, load_context.path()).await?;

    let mut materials = vec![];
    let mut named_materials = HashMap::new();
    let mut linear_textures = HashSet::new();
    for material in gltf.materials() {
        let handle = load_material(&material, load_context);
        if let Some(name) = material.name() {
            named_materials.insert(name.to_string(), handle.clone());
        }
        materials.push(handle);
        if let Some(texture) = material.normal_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material.occlusion_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            linear_textures.insert(texture.texture().index());
        }
    }

    let mut meshes = vec![];
    let mut named_meshes = HashMap::new();
    for mesh in gltf.meshes() {
        let mut primitives = vec![];
        for primitive in mesh.primitives() {
            let primitive_label = primitive_label(&mesh, &primitive);
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology);

            if let Some(vertex_attribute) = reader
                .read_positions()
                .map(|v| VertexAttributeValues::Float32x3(v.collect()))
            {
                mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_normals()
                .map(|v| VertexAttributeValues::Float32x3(v.collect()))
            {
                mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_tangents()
                .map(|v| VertexAttributeValues::Float32x4(v.collect()))
            {
                mesh.set_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_tex_coords(0)
                .map(|v| VertexAttributeValues::Float32x2(v.into_f32().collect()))
            {
                mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, vertex_attribute);
            } else {
                let len = mesh.count_vertices();
                let uvs = vec![[0.0, 0.0]; len];
                bevy_log::debug!("missing `TEXCOORD_0` vertex attribute, loading zeroed out UVs");
                mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            }

            if let Some(vertex_attribute) = reader
                .read_colors(0)
                .map(|v| VertexAttributeValues::Float32x4(v.into_rgba_f32().collect()))
            {
                mesh.set_attribute(Mesh::ATTRIBUTE_COLOR, vertex_attribute);
            }

            if let Some(indices) = reader.read_indices() {
                mesh.set_indices(Some(Indices::U32(indices.into_u32().collect())));
            };

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none() {
                let vertex_count_before = mesh.count_vertices();
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let vertex_count_after = mesh.count_vertices();

                if vertex_count_before != vertex_count_after {
                    bevy_log::debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
                } else {
                    bevy_log::debug!(
                        "Missing vertex normals in indexed geometry, computing them as flat."
                    );
                }
            }

            let mesh = load_context.set_labeled_asset(&primitive_label, LoadedAsset::new(mesh));
            primitives.push(super::GltfPrimitive {
                mesh,
                material: primitive
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned()),
            });
        }
        let handle = load_context.set_labeled_asset(
            &mesh_label(&mesh),
            LoadedAsset::new(super::GltfMesh { primitives }),
        );
        if let Some(name) = mesh.name() {
            named_meshes.insert(name.to_string(), handle.clone());
        }
        meshes.push(handle);
    }

    let mut nodes_intermediate = vec![];
    let mut named_nodes_intermediate = HashMap::new();
    for node in gltf.nodes() {
        let node_label = node_label(&node);
        nodes_intermediate.push((
            node_label,
            GltfNode {
                children: vec![],
                mesh: node
                    .mesh()
                    .map(|mesh| mesh.index())
                    .and_then(|i| meshes.get(i).cloned()),
                transform: match node.transform() {
                    gltf::scene::Transform::Matrix { matrix } => {
                        Transform::from_matrix(bevy_math::Mat4::from_cols_array_2d(&matrix))
                    }
                    gltf::scene::Transform::Decomposed {
                        translation,
                        rotation,
                        scale,
                    } => Transform {
                        translation: bevy_math::Vec3::from(translation),
                        rotation: bevy_math::Quat::from_vec4(rotation.into()),
                        scale: bevy_math::Vec3::from(scale),
                    },
                },
            },
            node.children()
                .map(|child| child.index())
                .collect::<Vec<_>>(),
        ));
        if let Some(name) = node.name() {
            named_nodes_intermediate.insert(name, node.index());
        }
    }
    let nodes = resolve_node_hierarchy(nodes_intermediate)
        .into_iter()
        .map(|(label, node)| load_context.set_labeled_asset(&label, LoadedAsset::new(node)))
        .collect::<Vec<bevy_asset::Handle<GltfNode>>>();
    let named_nodes = named_nodes_intermediate
        .into_iter()
        .filter_map(|(name, index)| {
            nodes
                .get(index)
                .map(|handle| (name.to_string(), handle.clone()))
        })
        .collect();

    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    #[cfg(target_arch = "wasm32")]
    for gltf_texture in gltf.textures() {
        let (texture, label) =
            load_texture(gltf_texture, &buffer_data, &linear_textures, &load_context).await?;
        load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
    }

    #[cfg(not(target_arch = "wasm32"))]
    load_context
        .task_pool()
        .scope(|scope| {
            gltf.textures().for_each(|gltf_texture| {
                let linear_textures = &linear_textures;
                let load_context: &LoadContext = load_context;
                let buffer_data = &buffer_data;
                scope.spawn(async move {
                    load_texture(gltf_texture, buffer_data, linear_textures, load_context).await
                });
            });
        })
        .into_iter()
        .filter_map(|result| result.ok())
        .for_each(|(texture, label)| {
            load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
        });

    let mut scenes = vec![];
    let mut named_scenes = HashMap::new();
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        world
            .spawn()
            .insert_bundle((Transform::identity(), GlobalTransform::identity()))
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
        let scene_handle = load_context
            .set_labeled_asset(&scene_label(&scene), LoadedAsset::new(Scene::new(world)));

        if let Some(name) = scene.name() {
            named_scenes.insert(name.to_string(), scene_handle.clone());
        }
        scenes.push(scene_handle);
    }

    load_context.set_default_asset(LoadedAsset::new(Gltf {
        default_scene: gltf
            .default_scene()
            .and_then(|scene| scenes.get(scene.index()))
            .cloned(),
        scenes,
        named_scenes,
        meshes,
        named_meshes,
        materials,
        named_materials,
        nodes,
        named_nodes,
    }));

    Ok(())
}

async fn load_texture<'a>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
    load_context: &LoadContext<'a>,
) -> Result<(Texture, String), GltfError> {
    let mut texture = match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset() as usize;
            let end = (view.offset() + view.length()) as usize;
            let buffer = &buffer_data[view.buffer().index()][start..end];
            Texture::from_buffer(buffer, ImageType::MimeType(mime_type))?
        }
        gltf::image::Source::Uri { uri, mime_type } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            let (bytes, image_type) = match DataUri::parse(uri) {
                Ok(data_uri) => (data_uri.decode()?, ImageType::MimeType(data_uri.mime_type)),
                Err(()) => {
                    let parent = load_context.path().parent().unwrap();
                    let image_path = parent.join(uri);
                    let bytes = load_context.read_asset_bytes(image_path.clone()).await?;

                    let extension = Path::new(uri).extension().unwrap().to_str().unwrap();
                    let image_type = ImageType::Extension(extension);

                    (bytes, image_type)
                }
            };

            Texture::from_buffer(
                &bytes,
                mime_type
                    .map(|mt| ImageType::MimeType(mt))
                    .unwrap_or(image_type),
            )?
        }
    };
    texture.sampler = texture_sampler(&gltf_texture);
    if (linear_textures).contains(&gltf_texture.index()) {
        texture.format = TextureFormat::Rgba8Unorm;
    }

    Ok((texture, texture_label(&gltf_texture)))
}

fn load_material(material: &Material, load_context: &mut LoadContext) -> Handle<StandardMaterial> {
    let material_label = material_label(&material);

    let pbr = material.pbr_metallic_roughness();

    let color = pbr.base_color_factor();
    let base_color_texture = if let Some(info) = pbr.base_color_texture() {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let normal_map = if let Some(normal_texture) = material.normal_texture() {
        // TODO: handle normal_texture.scale
        // TODO: handle normal_texture.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&normal_texture.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let metallic_roughness_texture = if let Some(info) = pbr.metallic_roughness_texture() {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let occlusion_texture = if let Some(occlusion_texture) = material.occlusion_texture() {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&occlusion_texture.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let emissive = material.emissive_factor();
    let emissive_texture = if let Some(info) = material.emissive_texture() {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    load_context.set_labeled_asset(
        &material_label,
        LoadedAsset::new(StandardMaterial {
            base_color: Color::rgba(color[0], color[1], color[2], color[3]),
            base_color_texture,
            roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_texture,
            normal_map,
            double_sided: material.double_sided(),
            occlusion_texture,
            emissive: Color::rgba(emissive[0], emissive[1], emissive[2], 1.0),
            emissive_texture,
            unlit: material.unlit(),
            ..Default::default()
        }),
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
    let mut node = world_builder.spawn_bundle((
        Transform::from_matrix(Mat4::from_cols_array_2d(&transform.matrix())),
        GlobalTransform::identity(),
    ));

    if let Some(name) = gltf_node.name() {
        node.insert(Name::new(name.to_string()));
    }

    // create camera node
    if let Some(camera) = gltf_node.camera() {
        node.insert(VisibleEntities {
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

                node.insert(Camera {
                    name: Some(base::camera::CAMERA_2D.to_owned()),
                    projection_matrix: orthographic_projection.get_projection_matrix(),
                    ..Default::default()
                });
                node.insert(orthographic_projection);
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
                node.insert(Camera {
                    name: Some(base::camera::CAMERA_3D.to_owned()),
                    projection_matrix: perspective_projection.get_projection_matrix(),
                    ..Default::default()
                });
                node.insert(perspective_projection);
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

                parent.spawn_bundle(PbrBundle {
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

fn mesh_label(mesh: &gltf::Mesh) -> String {
    format!("Mesh{}", mesh.index())
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

fn node_label(node: &gltf::Node) -> String {
    format!("Node{}", node.index())
}

fn scene_label(scene: &gltf::Scene) -> String {
    format!("Scene{}", scene.index())
}

fn texture_sampler(texture: &gltf::Texture) -> SamplerDescriptor {
    let gltf_sampler = texture.sampler();

    SamplerDescriptor {
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
                MinFilter::Nearest
                | MinFilter::NearestMipmapNearest
                | MinFilter::NearestMipmapLinear => FilterMode::Nearest,
                MinFilter::Linear
                | MinFilter::LinearMipmapNearest
                | MinFilter::LinearMipmapLinear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().min_filter),

        mipmap_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::Linear
                | MinFilter::NearestMipmapNearest
                | MinFilter::LinearMipmapNearest => FilterMode::Nearest,
                MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                    FilterMode::Linear
                }
            })
            .unwrap_or(SamplerDescriptor::default().mipmap_filter),

        ..Default::default()
    }
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
    const OCTET_STREAM_URI: &str = "application/octet-stream";

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) if data_uri.mime_type == OCTET_STREAM_URI => data_uri.decode()?,
                    Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                    Err(()) => {
                        // TODO: Remove this and add dep
                        let buffer_path = asset_path.parent().unwrap().join(uri);
                        let buffer_bytes = load_context.read_asset_bytes(buffer_path).await?;
                        buffer_bytes
                    }
                };
                buffer_data.push(buffer_bytes);
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

fn resolve_node_hierarchy(
    nodes_intermediate: Vec<(String, GltfNode, Vec<usize>)>,
) -> Vec<(String, GltfNode)> {
    let mut max_steps = nodes_intermediate.len();
    let mut nodes_step = nodes_intermediate
        .into_iter()
        .enumerate()
        .map(|(i, (label, node, children))| (i, label, node, children))
        .collect::<Vec<_>>();
    let mut nodes = std::collections::HashMap::<usize, (String, GltfNode)>::new();
    while max_steps > 0 && !nodes_step.is_empty() {
        if let Some((index, label, node, _)) = nodes_step
            .iter()
            .find(|(_, _, _, children)| children.is_empty())
            .cloned()
        {
            nodes.insert(index, (label, node));
            for (_, _, node, children) in nodes_step.iter_mut() {
                if let Some((i, _)) = children
                    .iter()
                    .enumerate()
                    .find(|(_, child_index)| **child_index == index)
                {
                    children.remove(i);

                    if let Some((_, child_node)) = nodes.get(&index) {
                        node.children.push(child_node.clone())
                    }
                }
            }
            nodes_step = nodes_step
                .into_iter()
                .filter(|(i, _, _, _)| *i != index)
                .collect()
        }
        max_steps -= 1;
    }

    let mut nodes_to_sort = nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect()
}

struct DataUri<'a> {
    mime_type: &'a str,
    base64: bool,
    data: &'a str,
}

fn split_once(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, delimiter);
    Some((iter.next()?, iter.next()?))
}

impl<'a> DataUri<'a> {
    fn parse(uri: &'a str) -> Result<DataUri<'a>, ()> {
        let uri = uri.strip_prefix("data:").ok_or(())?;
        let (mime_type, data) = split_once(uri, ',').ok_or(())?;

        let (mime_type, base64) = match mime_type.strip_suffix(";base64") {
            Some(mime_type) => (mime_type, true),
            None => (mime_type, false),
        };

        Ok(DataUri {
            mime_type,
            base64,
            data,
        })
    }

    fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        if self.base64 {
            base64::decode(self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

#[cfg(test)]
mod test {
    use super::resolve_node_hierarchy;
    use crate::GltfNode;

    impl GltfNode {
        fn empty() -> Self {
            GltfNode {
                children: vec![],
                mesh: None,
                transform: bevy_transform::prelude::Transform::identity(),
            }
        }
    }
    #[test]
    fn node_hierarchy_single_node() {
        let result = resolve_node_hierarchy(vec![("l1".to_string(), GltfNode::empty(), vec![])]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_no_hierarchy() {
        let result = resolve_node_hierarchy(vec![
            ("l1".to_string(), GltfNode::empty(), vec![]),
            ("l2".to_string(), GltfNode::empty(), vec![]),
        ]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_simple_hierarchy() {
        let result = resolve_node_hierarchy(vec![
            ("l1".to_string(), GltfNode::empty(), vec![1]),
            ("l2".to_string(), GltfNode::empty(), vec![]),
        ]);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 1);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_hierarchy() {
        let result = resolve_node_hierarchy(vec![
            ("l1".to_string(), GltfNode::empty(), vec![1]),
            ("l2".to_string(), GltfNode::empty(), vec![2]),
            ("l3".to_string(), GltfNode::empty(), vec![3, 4, 5]),
            ("l4".to_string(), GltfNode::empty(), vec![6]),
            ("l5".to_string(), GltfNode::empty(), vec![]),
            ("l6".to_string(), GltfNode::empty(), vec![]),
            ("l7".to_string(), GltfNode::empty(), vec![]),
        ]);

        assert_eq!(result.len(), 7);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 1);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 1);
        assert_eq!(result[2].0, "l3");
        assert_eq!(result[2].1.children.len(), 3);
        assert_eq!(result[3].0, "l4");
        assert_eq!(result[3].1.children.len(), 1);
        assert_eq!(result[4].0, "l5");
        assert_eq!(result[4].1.children.len(), 0);
        assert_eq!(result[5].0, "l6");
        assert_eq!(result[5].1.children.len(), 0);
        assert_eq!(result[6].0, "l7");
        assert_eq!(result[6].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_cyclic() {
        let result = resolve_node_hierarchy(vec![
            ("l1".to_string(), GltfNode::empty(), vec![1]),
            ("l2".to_string(), GltfNode::empty(), vec![0]),
        ]);

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn node_hierarchy_missing_node() {
        let result = resolve_node_hierarchy(vec![
            ("l1".to_string(), GltfNode::empty(), vec![2]),
            ("l2".to_string(), GltfNode::empty(), vec![]),
        ]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l2");
        assert_eq!(result[0].1.children.len(), 0);
    }
}
