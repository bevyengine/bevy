use crate::{vertex_attributes::convert_attribute, Gltf, GltfExtras, GltfNode};
use bevy_asset::{
    io::Reader, AssetLoadError, AssetLoader, AsyncReadExt, Handle, LoadContext, ReadAssetBytesError,
};
use bevy_core::Name;
use bevy_core_pipeline::prelude::Camera3dBundle;
use bevy_ecs::{entity::Entity, world::World};
use bevy_hierarchy::{BuildWorldChildren, WorldChildBuilder};
use bevy_log::{error, warn};
use bevy_math::{Mat4, Vec3};
use bevy_pbr::{
    AlphaMode, DirectionalLight, DirectionalLightBundle, PbrBundle, PointLight, PointLightBundle,
    SpotLight, SpotLightBundle, StandardMaterial, MAX_JOINTS,
};
use bevy_render::{
    camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode},
    color::Color,
    mesh::{
        morph::{MeshMorphWeights, MorphAttributes, MorphTargetImage, MorphWeights},
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Indices, Mesh, MeshVertexAttribute, VertexAttributeValues,
    },
    prelude::SpatialBundle,
    primitives::Aabb,
    render_resource::{Face, PrimitiveTopology},
    texture::{
        CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageLoaderSettings,
        ImageSampler, ImageSamplerDescriptor, ImageType, TextureError,
    },
};
use bevy_scene::Scene;
#[cfg(not(target_arch = "wasm32"))]
use bevy_tasks::IoTaskPool;
use bevy_transform::components::Transform;
use bevy_utils::{HashMap, HashSet};
use gltf::{
    accessor::Iter,
    mesh::{util::ReadIndices, Mode},
    texture::{MagFilter, MinFilter, WrappingMode},
    Material, Node, Primitive, Semantic,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// An error that occurs when loading a glTF file.
#[derive(Error, Debug)]
pub enum GltfError {
    /// Unsupported primitive mode.
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive {
        /// The primitive mode.
        mode: Mode,
    },
    /// Invalid glTF file.
    #[error("invalid glTF file: {0}")]
    Gltf(#[from] gltf::Error),
    /// Binary blob is missing.
    #[error("binary blob is missing")]
    MissingBlob,
    /// Decoding the base64 mesh data failed.
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    /// Unsupported buffer format.
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    /// Invalid image mime type.
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    /// Error when loading a texture. Might be due to a disabled image file format feature.
    #[error("You may need to add the feature for the file format: {0}")]
    ImageError(#[from] TextureError),
    /// Failed to read bytes from an asset path.
    #[error("failed to read bytes from an asset path: {0}")]
    ReadAssetBytesError(#[from] ReadAssetBytesError),
    /// Failed to load asset from an asset path.
    #[error("failed to load asset from an asset path: {0}")]
    AssetLoadError(#[from] AssetLoadError),
    /// Missing sampler for an animation.
    #[error("Missing sampler for animation {0}")]
    MissingAnimationSampler(usize),
    /// Failed to generate tangents.
    #[error("failed to generate tangents: {0}")]
    GenerateTangentsError(#[from] bevy_render::mesh::GenerateTangentsError),
    /// Failed to generate morph targets.
    #[error("failed to generate morph targets: {0}")]
    MorphTarget(#[from] bevy_render::mesh::morph::MorphBuildError),
    /// Failed to load a file.
    #[error("failed to load file: {0}")]
    Io(#[from] std::io::Error),
}

/// Loads glTF files with all of their data as their corresponding bevy representations.
pub struct GltfLoader {
    /// List of compressed image formats handled by the loader.
    pub supported_compressed_formats: CompressedImageFormats,
    /// Custom vertex attributes that will be recognized when loading a glTF file.
    ///
    /// Keys must be the attribute names as found in the glTF data, which must start with an underscore.
    /// See [this section of the glTF specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#meshes-overview)
    /// for additional details on custom attributes.
    pub custom_vertex_attributes: HashMap<String, MeshVertexAttribute>,
}

/// Specifies optional settings for processing gltfs at load time. By default, all recognized contents of
/// the gltf will be loaded.
///
/// # Example
///
/// To load a gltf but exclude the cameras, replace a call to `asset_server.load("my.gltf")` with
/// ```no_run
/// # use bevy_asset::{AssetServer, Handle};
/// # use bevy_gltf::*;
/// # let asset_server: AssetServer = panic!();
/// let gltf_handle: Handle<Gltf> = asset_server.load_with_settings(
///     "my.gltf",
///     |s: &mut GltfLoaderSettings| {
///         s.load_cameras = false;
///     }
/// );    
/// ```
#[derive(Serialize, Deserialize)]
pub struct GltfLoaderSettings {
    /// If true, the loader will load mesh nodes and the associated materials.
    pub load_meshes: bool,
    /// If true, the loader will spawn cameras for gltf camera nodes.
    pub load_cameras: bool,
    /// If true, the loader will spawn lights for gltf light nodes.
    pub load_lights: bool,
}

impl Default for GltfLoaderSettings {
    fn default() -> Self {
        Self {
            load_meshes: true,
            load_cameras: true,
            load_lights: true,
        }
    }
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = GltfLoaderSettings;
    type Error = GltfError;
    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        settings: &'a GltfLoaderSettings,
        load_context: &'a mut LoadContext,
    ) -> bevy_utils::BoxedFuture<'a, Result<Gltf, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            load_gltf(self, &bytes, load_context, settings).await
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

/// Loads an entire glTF file.
async fn load_gltf<'a, 'b, 'c>(
    loader: &GltfLoader,
    bytes: &'a [u8],
    load_context: &'b mut LoadContext<'c>,
    settings: &'b GltfLoaderSettings,
) -> Result<Gltf, GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let buffer_data = load_buffers(&gltf, load_context).await?;

    let mut linear_textures = HashSet::default();

    for material in gltf.materials() {
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

    #[cfg(feature = "bevy_animation")]
    let paths = {
        let mut paths = HashMap::<usize, (usize, Vec<Name>)>::new();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                let root_index = node.index();
                paths_recur(node, &[], &mut paths, root_index);
            }
        }
        paths
    };

    #[cfg(feature = "bevy_animation")]
    let (animations, named_animations, animation_roots) = {
        use bevy_animation::{Interpolation, Keyframes};
        use gltf::animation::util::ReadOutputs;
        let mut animations = vec![];
        let mut named_animations = HashMap::default();
        let mut animation_roots = HashSet::default();
        for animation in gltf.animations() {
            let mut animation_clip = bevy_animation::AnimationClip::default();
            for channel in animation.channels() {
                let interpolation = match channel.sampler().interpolation() {
                    gltf::animation::Interpolation::Linear => Interpolation::Linear,
                    gltf::animation::Interpolation::Step => Interpolation::Step,
                    gltf::animation::Interpolation::CubicSpline => Interpolation::CubicSpline,
                };
                let node = channel.target().node();
                let reader = channel.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let keyframe_timestamps: Vec<f32> = if let Some(inputs) = reader.read_inputs() {
                    match inputs {
                        Iter::Standard(times) => times.collect(),
                        Iter::Sparse(_) => {
                            warn!("Sparse accessor not supported for animation sampler input");
                            continue;
                        }
                    }
                } else {
                    warn!("Animations without a sampler input are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                let keyframes = if let Some(outputs) = reader.read_outputs() {
                    match outputs {
                        ReadOutputs::Translations(tr) => {
                            Keyframes::Translation(tr.map(Vec3::from).collect())
                        }
                        ReadOutputs::Rotations(rots) => Keyframes::Rotation(
                            rots.into_f32().map(bevy_math::Quat::from_array).collect(),
                        ),
                        ReadOutputs::Scales(scale) => {
                            Keyframes::Scale(scale.map(Vec3::from).collect())
                        }
                        ReadOutputs::MorphTargetWeights(weights) => {
                            Keyframes::Weights(weights.into_f32().collect())
                        }
                    }
                } else {
                    warn!("Animations without a sampler output are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                if let Some((root_index, path)) = paths.get(&node.index()) {
                    animation_roots.insert(root_index);
                    animation_clip.add_curve_to_path(
                        bevy_animation::EntityPath {
                            parts: path.clone(),
                        },
                        bevy_animation::VariableCurve {
                            keyframe_timestamps,
                            keyframes,
                            interpolation,
                        },
                    );
                } else {
                    warn!(
                        "Animation ignored for node {}: part of its hierarchy is missing a name",
                        node.index()
                    );
                }
            }
            let handle = load_context
                .add_labeled_asset(format!("Animation{}", animation.index()), animation_clip);
            if let Some(name) = animation.name() {
                named_animations.insert(name.to_string(), handle.clone());
            }
            animations.push(handle);
        }
        (animations, named_animations, animation_roots)
    };

    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    // See https://github.com/bevyengine/bevy/issues/1924 for more details
    // The taskpool use is also avoided when there is only one texture for performance reasons and
    // to avoid https://github.com/bevyengine/bevy/pull/2725
    // PERF: could this be a Vec instead? Are gltf texture indices dense?
    fn process_loaded_texture(
        load_context: &mut LoadContext,
        handles: &mut Vec<Handle<Image>>,
        texture: ImageOrPath,
    ) {
        let handle = match texture {
            ImageOrPath::Image { label, image } => load_context.add_labeled_asset(label, image),
            ImageOrPath::Path {
                path,
                is_srgb,
                sampler_descriptor,
            } => {
                load_context.load_with_settings(path, move |settings: &mut ImageLoaderSettings| {
                    settings.is_srgb = is_srgb;
                    settings.sampler = ImageSampler::Descriptor(sampler_descriptor.clone());
                })
            }
        };
        handles.push(handle);
    }

    // We collect handles to ensure loaded images from paths are not unloaded before they are used elsewhere
    // in the loader. This prevents "reloads", but it also prevents dropping the is_srgb context on reload.
    //
    // In theory we could store a mapping between texture.index() and handle to use
    // later in the loader when looking up handles for materials. However this would mean
    // that the material's load context would no longer track those images as dependencies.
    let mut _texture_handles = Vec::new();
    if gltf.textures().len() == 1 || cfg!(target_arch = "wasm32") {
        for texture in gltf.textures() {
            let parent_path = load_context.path().parent().unwrap();
            let image = load_image(
                texture,
                &buffer_data,
                &linear_textures,
                parent_path,
                loader.supported_compressed_formats,
            )
            .await?;
            process_loaded_texture(load_context, &mut _texture_handles, image);
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        IoTaskPool::get()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let parent_path = load_context.path().parent().unwrap();
                    let linear_textures = &linear_textures;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        load_image(
                            gltf_texture,
                            buffer_data,
                            linear_textures,
                            parent_path,
                            loader.supported_compressed_formats,
                        )
                        .await
                    });
                });
            })
            .into_iter()
            .for_each(|result| match result {
                Ok(image) => {
                    process_loaded_texture(load_context, &mut _texture_handles, image);
                }
                Err(err) => {
                    warn!("Error loading glTF texture: {}", err);
                }
            });
    }

    let mut materials = vec![];
    let mut named_materials = HashMap::default();
    // NOTE: materials must be loaded after textures because image load() calls will happen before load_with_settings, preventing is_srgb from being set properly
    for material in gltf.materials() {
        let handle = load_material(&material, load_context, false);
        if let Some(name) = material.name() {
            named_materials.insert(name.to_string(), handle.clone());
        }
        materials.push(handle);
    }

    let mut meshes = vec![];
    let mut named_meshes = HashMap::default();
    let mut meshes_on_skinned_nodes = HashSet::default();
    let mut meshes_on_non_skinned_nodes = HashSet::default();
    for gltf_node in gltf.nodes() {
        if gltf_node.skin().is_some() {
            if let Some(mesh) = gltf_node.mesh() {
                meshes_on_skinned_nodes.insert(mesh.index());
            }
        } else if let Some(mesh) = gltf_node.mesh() {
            meshes_on_non_skinned_nodes.insert(mesh.index());
        }
    }
    for gltf_mesh in gltf.meshes() {
        let mut primitives = vec![];
        for primitive in gltf_mesh.primitives() {
            let primitive_label = primitive_label(&gltf_mesh, &primitive);
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology);

            // Read vertex attributes
            for (semantic, accessor) in primitive.attributes() {
                if [Semantic::Joints(0), Semantic::Weights(0)].contains(&semantic) {
                    if !meshes_on_skinned_nodes.contains(&gltf_mesh.index()) {
                        warn!(
                        "Ignoring attribute {:?} for skinned mesh {:?} used on non skinned nodes (NODE_SKINNED_MESH_WITHOUT_SKIN)",
                        semantic,
                        primitive_label
                    );
                        continue;
                    } else if meshes_on_non_skinned_nodes.contains(&gltf_mesh.index()) {
                        error!("Skinned mesh {:?} used on both skinned and non skin nodes, this is likely to cause an error (NODE_SKINNED_MESH_WITHOUT_SKIN)", primitive_label);
                    }
                }
                match convert_attribute(
                    semantic,
                    accessor,
                    &buffer_data,
                    &loader.custom_vertex_attributes,
                ) {
                    Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
                    Err(err) => warn!("{}", err),
                }
            }

            // Read vertex indices
            let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].as_slice()));
            if let Some(indices) = reader.read_indices() {
                mesh.set_indices(Some(match indices {
                    ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
                    ReadIndices::U16(is) => Indices::U16(is.collect()),
                    ReadIndices::U32(is) => Indices::U32(is.collect()),
                }));
            };

            {
                let morph_target_reader = reader.read_morph_targets();
                if morph_target_reader.len() != 0 {
                    let morph_targets_label = morph_targets_label(&gltf_mesh, &primitive);
                    let morph_target_image = MorphTargetImage::new(
                        morph_target_reader.map(PrimitiveMorphAttributesIter),
                        mesh.count_vertices(),
                    )?;
                    let handle =
                        load_context.add_labeled_asset(morph_targets_label, morph_target_image.0);

                    mesh.set_morph_targets(handle);
                    let extras = gltf_mesh.extras().as_ref();
                    if let Option::<MorphTargetNames>::Some(names) =
                        extras.and_then(|extras| serde_json::from_str(extras.get()).ok())
                    {
                        mesh.set_morph_target_names(names.target_names);
                    }
                }
            }

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
                && matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
            {
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

            if let Some(vertex_attribute) = reader
                .read_tangents()
                .map(|v| VertexAttributeValues::Float32x4(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
            } else if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some()
                && primitive.material().normal_texture().is_some()
            {
                bevy_log::debug!(
                    "Missing vertex tangents, computing them using the mikktspace algorithm"
                );
                if let Err(err) = mesh.generate_tangents() {
                    warn!(
                        "Failed to generate vertex tangents using the mikktspace algorithm: {:?}",
                        err
                    );
                }
            }

            let mesh = load_context.add_labeled_asset(primitive_label, mesh);
            primitives.push(super::GltfPrimitive {
                mesh,
                material: primitive
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned()),
                extras: get_gltf_extras(primitive.extras()),
                material_extras: get_gltf_extras(primitive.material().extras()),
            });
        }

        let handle = load_context.add_labeled_asset(
            mesh_label(&gltf_mesh),
            super::GltfMesh {
                primitives,
                extras: get_gltf_extras(gltf_mesh.extras()),
            },
        );
        if let Some(name) = gltf_mesh.name() {
            named_meshes.insert(name.to_string(), handle.clone());
        }
        meshes.push(handle);
    }

    let mut nodes_intermediate = vec![];
    let mut named_nodes_intermediate = HashMap::default();
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
                        Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
                    }
                    gltf::scene::Transform::Decomposed {
                        translation,
                        rotation,
                        scale,
                    } => Transform {
                        translation: bevy_math::Vec3::from(translation),
                        rotation: bevy_math::Quat::from_array(rotation),
                        scale: bevy_math::Vec3::from(scale),
                    },
                },
                extras: get_gltf_extras(node.extras()),
            },
            node.children()
                .map(|child| child.index())
                .collect::<Vec<_>>(),
        ));
        if let Some(name) = node.name() {
            named_nodes_intermediate.insert(name, node.index());
        }
    }
    let nodes = resolve_node_hierarchy(nodes_intermediate, load_context.path())
        .into_iter()
        .map(|(label, node)| load_context.add_labeled_asset(label, node))
        .collect::<Vec<Handle<GltfNode>>>();
    let named_nodes = named_nodes_intermediate
        .into_iter()
        .filter_map(|(name, index)| {
            nodes
                .get(index)
                .map(|handle| (name.to_string(), handle.clone()))
        })
        .collect();

    let skinned_mesh_inverse_bindposes: Vec<_> = gltf
        .skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let inverse_bindposes: Vec<Mat4> = reader
                .read_inverse_bind_matrices()
                .unwrap()
                .map(|mat| Mat4::from_cols_array_2d(&mat))
                .collect();

            load_context.add_labeled_asset(
                skin_label(&gltf_skin),
                SkinnedMeshInverseBindposes::from(inverse_bindposes),
            )
        })
        .collect();

    let mut scenes = vec![];
    let mut named_scenes = HashMap::default();
    let mut active_camera_found = false;
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        let mut node_index_to_entity_map = HashMap::new();
        let mut entity_to_skin_index_map = HashMap::new();
        let mut scene_load_context = load_context.begin_labeled_asset();
        world
            .spawn(SpatialBundle::INHERITED_IDENTITY)
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = load_node(
                        &node,
                        parent,
                        load_context,
                        &mut scene_load_context,
                        settings,
                        &mut node_index_to_entity_map,
                        &mut entity_to_skin_index_map,
                        &mut active_camera_found,
                        &Transform::default(),
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            });
        if let Some(Err(err)) = err {
            return Err(err);
        }

        #[cfg(feature = "bevy_animation")]
        {
            // for each node root in a scene, check if it's the root of an animation
            // if it is, add the AnimationPlayer component
            for node in scene.nodes() {
                if animation_roots.contains(&node.index()) {
                    world
                        .entity_mut(*node_index_to_entity_map.get(&node.index()).unwrap())
                        .insert(bevy_animation::AnimationPlayer::default());
                }
            }
        }

        let mut warned_about_max_joints = HashSet::new();
        for (&entity, &skin_index) in &entity_to_skin_index_map {
            let mut entity = world.entity_mut(entity);
            let skin = gltf.skins().nth(skin_index).unwrap();
            let joint_entities: Vec<_> = skin
                .joints()
                .map(|node| node_index_to_entity_map[&node.index()])
                .collect();

            if joint_entities.len() > MAX_JOINTS && warned_about_max_joints.insert(skin_index) {
                warn!(
                    "The glTF skin {:?} has {} joints, but the maximum supported is {}",
                    skin.name()
                        .map(|name| name.to_string())
                        .unwrap_or_else(|| skin.index().to_string()),
                    joint_entities.len(),
                    MAX_JOINTS
                );
            }
            entity.insert(SkinnedMesh {
                inverse_bindposes: skinned_mesh_inverse_bindposes[skin_index].clone(),
                joints: joint_entities,
            });
        }
        let loaded_scene = scene_load_context.finish(Scene::new(world), None);
        let scene_handle = load_context.add_loaded_labeled_asset(scene_label(&scene), loaded_scene);

        if let Some(name) = scene.name() {
            named_scenes.insert(name.to_string(), scene_handle.clone());
        }
        scenes.push(scene_handle);
    }

    Ok(Gltf {
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
        #[cfg(feature = "bevy_animation")]
        animations,
        #[cfg(feature = "bevy_animation")]
        named_animations,
    })
}

fn get_gltf_extras(extras: &gltf::json::Extras) -> Option<GltfExtras> {
    extras.as_ref().map(|extras| GltfExtras {
        value: extras.get().to_string(),
    })
}

fn node_name(node: &Node) -> Name {
    let name = node
        .name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("GltfNode{}", node.index()));
    Name::new(name)
}

#[cfg(feature = "bevy_animation")]
fn paths_recur(
    node: Node,
    current_path: &[Name],
    paths: &mut HashMap<usize, (usize, Vec<Name>)>,
    root_index: usize,
) {
    let mut path = current_path.to_owned();
    path.push(node_name(&node));
    for child in node.children() {
        paths_recur(child, &path, paths, root_index);
    }
    paths.insert(node.index(), (root_index, path));
}

/// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
async fn load_image<'a, 'b>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
    parent_path: &'b Path,
    supported_compressed_formats: CompressedImageFormats,
) -> Result<ImageOrPath, GltfError> {
    let is_srgb = !linear_textures.contains(&gltf_texture.index());
    let sampler_descriptor = texture_sampler(&gltf_texture);
    match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset();
            let end = view.offset() + view.length();
            let buffer = &buffer_data[view.buffer().index()][start..end];
            let image = Image::from_buffer(
                buffer,
                ImageType::MimeType(mime_type),
                supported_compressed_formats,
                is_srgb,
                ImageSampler::Descriptor(sampler_descriptor),
            )?;
            Ok(ImageOrPath::Image {
                image,
                label: texture_label(&gltf_texture),
            })
        }
        gltf::image::Source::Uri { uri, mime_type } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            if let Ok(data_uri) = DataUri::parse(uri) {
                let bytes = data_uri.decode()?;
                let image_type = ImageType::MimeType(data_uri.mime_type);
                Ok(ImageOrPath::Image {
                    image: Image::from_buffer(
                        &bytes,
                        mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                        supported_compressed_formats,
                        is_srgb,
                        ImageSampler::Descriptor(sampler_descriptor),
                    )?,
                    label: texture_label(&gltf_texture),
                })
            } else {
                let image_path = parent_path.join(uri);
                Ok(ImageOrPath::Path {
                    path: image_path,
                    is_srgb,
                    sampler_descriptor,
                })
            }
        }
    }
}

/// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
fn load_material(
    material: &Material,
    load_context: &mut LoadContext,
    is_scale_inverted: bool,
) -> Handle<StandardMaterial> {
    let material_label = material_label(material, is_scale_inverted);
    load_context.labeled_asset_scope(material_label, |load_context| {
        let pbr = material.pbr_metallic_roughness();

        // TODO: handle missing label handle errors here?
        let color = pbr.base_color_factor();
        let base_color_texture = pbr.base_color_texture().map(|info| {
            // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
            texture_handle(load_context, &info.texture())
        });

        let normal_map_texture: Option<Handle<Image>> =
            material.normal_texture().map(|normal_texture| {
                // TODO: handle normal_texture.scale
                // TODO: handle normal_texture.tex_coord() (the *set* index for the right texcoords)
                texture_handle(load_context, &normal_texture.texture())
            });

        let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
            // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
            texture_handle(load_context, &info.texture())
        });

        let occlusion_texture = material.occlusion_texture().map(|occlusion_texture| {
            // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            texture_handle(load_context, &occlusion_texture.texture())
        });

        let emissive = material.emissive_factor();
        let emissive_texture = material.emissive_texture().map(|info| {
            // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            texture_handle(load_context, &info.texture())
        });

        #[cfg(feature = "pbr_transmission_textures")]
        let (specular_transmission, specular_transmission_texture) =
            material.transmission().map_or((0.0, None), |transmission| {
                let transmission_texture: Option<Handle<Image>> = transmission
                    .transmission_texture()
                    .map(|transmission_texture| {
                        // TODO: handle transmission_texture.tex_coord() (the *set* index for the right texcoords)
                        texture_handle(load_context, &transmission_texture.texture())
                    });

                (transmission.transmission_factor(), transmission_texture)
            });

        #[cfg(not(feature = "pbr_transmission_textures"))]
        let specular_transmission = material
            .transmission()
            .map_or(0.0, |transmission| transmission.transmission_factor());

        #[cfg(feature = "pbr_transmission_textures")]
        let (thickness, thickness_texture, attenuation_distance, attenuation_color) = material
            .volume()
            .map_or((0.0, None, f32::INFINITY, [1.0, 1.0, 1.0]), |volume| {
                let thickness_texture: Option<Handle<Image>> =
                    volume.thickness_texture().map(|thickness_texture| {
                        // TODO: handle thickness_texture.tex_coord() (the *set* index for the right texcoords)
                        texture_handle(load_context, &thickness_texture.texture())
                    });

                (
                    volume.thickness_factor(),
                    thickness_texture,
                    volume.attenuation_distance(),
                    volume.attenuation_color(),
                )
            });

        #[cfg(not(feature = "pbr_transmission_textures"))]
        let (thickness, attenuation_distance, attenuation_color) =
            material
                .volume()
                .map_or((0.0, f32::INFINITY, [1.0, 1.0, 1.0]), |volume| {
                    (
                        volume.thickness_factor(),
                        volume.attenuation_distance(),
                        volume.attenuation_color(),
                    )
                });

        let ior = material.ior().unwrap_or(1.5);

        StandardMaterial {
            base_color: Color::rgba_linear(color[0], color[1], color[2], color[3]),
            base_color_texture,
            perceptual_roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_texture,
            normal_map_texture,
            double_sided: material.double_sided(),
            cull_mode: if material.double_sided() {
                None
            } else if is_scale_inverted {
                Some(Face::Front)
            } else {
                Some(Face::Back)
            },
            occlusion_texture,
            emissive: Color::rgb_linear(emissive[0], emissive[1], emissive[2])
                * material.emissive_strength().unwrap_or(1.0),
            emissive_texture,
            specular_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture,
            thickness,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture,
            ior,
            attenuation_distance,
            attenuation_color: Color::rgb_linear(
                attenuation_color[0],
                attenuation_color[1],
                attenuation_color[2],
            ),
            unlit: material.unlit(),
            alpha_mode: alpha_mode(material),
            ..Default::default()
        }
    })
}

/// Loads a glTF node.
#[allow(clippy::too_many_arguments, clippy::result_large_err)]
fn load_node(
    gltf_node: &Node,
    world_builder: &mut WorldChildBuilder,
    root_load_context: &LoadContext,
    load_context: &mut LoadContext,
    settings: &GltfLoaderSettings,
    node_index_to_entity_map: &mut HashMap<usize, Entity>,
    entity_to_skin_index_map: &mut HashMap<Entity, usize>,
    active_camera_found: &mut bool,
    parent_transform: &Transform,
) -> Result<(), GltfError> {
    let transform = gltf_node.transform();
    let mut gltf_error = None;
    let transform = Transform::from_matrix(Mat4::from_cols_array_2d(&transform.matrix()));
    let world_transform = *parent_transform * transform;
    // according to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#instantiation,
    // if the determinant of the transform is negative we must invert the winding order of
    // triangles in meshes on the node.
    // instead we equivalently test if the global scale is inverted by checking if the number
    // of negative scale factors is odd. if so we will assign a copy of the material with face
    // culling inverted, rather than modifying the mesh data directly.
    let is_scale_inverted = world_transform.scale.is_negative_bitmask().count_ones() & 1 == 1;
    let mut node = world_builder.spawn(SpatialBundle::from(transform));

    node.insert(node_name(gltf_node));

    if let Some(extras) = gltf_node.extras() {
        node.insert(GltfExtras {
            value: extras.get().to_string(),
        });
    }

    // create camera node
    if settings.load_cameras {
        if let Some(camera) = gltf_node.camera() {
            let projection = match camera.projection() {
                gltf::camera::Projection::Orthographic(orthographic) => {
                    let xmag = orthographic.xmag();
                    let orthographic_projection = OrthographicProjection {
                        near: orthographic.znear(),
                        far: orthographic.zfar(),
                        scaling_mode: ScalingMode::FixedHorizontal(1.0),
                        scale: xmag,
                        ..Default::default()
                    };

                    Projection::Orthographic(orthographic_projection)
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
                    Projection::Perspective(perspective_projection)
                }
            };
            node.insert(Camera3dBundle {
                projection,
                transform,
                camera: Camera {
                    is_active: !*active_camera_found,
                    ..Default::default()
                },
                ..Default::default()
            });

            *active_camera_found = true;
        }
    }

    // Map node index to entity
    node_index_to_entity_map.insert(gltf_node.index(), node.id());

    let mut morph_weights = None;

    node.with_children(|parent| {
        if settings.load_meshes {
            if let Some(mesh) = gltf_node.mesh() {
                // append primitives
                for primitive in mesh.primitives() {
                    let material = primitive.material();
                    let material_label = material_label(&material, is_scale_inverted);

                    // This will make sure we load the default material now since it would not have been
                    // added when iterating over all the gltf materials (since the default material is
                    // not explicitly listed in the gltf).
                    // It also ensures an inverted scale copy is instantiated if required.
                    if !root_load_context.has_labeled_asset(&material_label)
                        && !load_context.has_labeled_asset(&material_label)
                    {
                        load_material(&material, load_context, is_scale_inverted);
                    }

                    let primitive_label = primitive_label(&mesh, &primitive);
                    let bounds = primitive.bounding_box();

                    let mut mesh_entity = parent.spawn(PbrBundle {
                        // TODO: handle missing label handle errors here?
                        mesh: load_context.get_label_handle(&primitive_label),
                        material: load_context.get_label_handle(&material_label),
                        ..Default::default()
                    });
                    let target_count = primitive.morph_targets().len();
                    if target_count != 0 {
                        let weights = match mesh.weights() {
                            Some(weights) => weights.to_vec(),
                            None => vec![0.0; target_count],
                        };

                        if morph_weights.is_none() {
                            morph_weights = Some(weights.clone());
                        }

                        // unwrap: the parent's call to `MeshMorphWeights::new`
                        // means this code doesn't run if it returns an `Err`.
                        // According to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets
                        // they should all have the same length.
                        // > All morph target accessors MUST have the same count as
                        // > the accessors of the original primitive.
                        mesh_entity.insert(MeshMorphWeights::new(weights).unwrap());
                    }
                    mesh_entity.insert(Aabb::from_min_max(
                        Vec3::from_slice(&bounds.min),
                        Vec3::from_slice(&bounds.max),
                    ));

                    if let Some(extras) = primitive.extras() {
                        mesh_entity.insert(GltfExtras {
                            value: extras.get().to_string(),
                        });
                    }

                    mesh_entity.insert(Name::new(primitive_name(&mesh, &primitive)));
                    // Mark for adding skinned mesh
                    if let Some(skin) = gltf_node.skin() {
                        entity_to_skin_index_map.insert(mesh_entity.id(), skin.index());
                    }
                }
            }
        }

        if settings.load_lights {
            if let Some(light) = gltf_node.light() {
                match light.kind() {
                    gltf::khr_lights_punctual::Kind::Directional => {
                        let mut entity = parent.spawn(DirectionalLightBundle {
                            directional_light: DirectionalLight {
                                color: Color::rgb_from_array(light.color()),
                                // NOTE: KHR_punctual_lights defines the intensity units for directional
                                // lights in lux (lm/m^2) which is what we need.
                                illuminance: light.intensity(),
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                    gltf::khr_lights_punctual::Kind::Point => {
                        let mut entity = parent.spawn(PointLightBundle {
                            point_light: PointLight {
                                color: Color::rgb_from_array(light.color()),
                                // NOTE: KHR_punctual_lights defines the intensity units for point lights in
                                // candela (lm/sr) which is luminous intensity and we need luminous power.
                                // For a point light, luminous power = 4 * pi * luminous intensity
                                intensity: light.intensity() * std::f32::consts::PI * 4.0,
                                range: light.range().unwrap_or(20.0),
                                radius: 0.0,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                    gltf::khr_lights_punctual::Kind::Spot {
                        inner_cone_angle,
                        outer_cone_angle,
                    } => {
                        let mut entity = parent.spawn(SpotLightBundle {
                            spot_light: SpotLight {
                                color: Color::rgb_from_array(light.color()),
                                // NOTE: KHR_punctual_lights defines the intensity units for spot lights in
                                // candela (lm/sr) which is luminous intensity and we need luminous power.
                                // For a spot light, we map luminous power = 4 * pi * luminous intensity
                                intensity: light.intensity() * std::f32::consts::PI * 4.0,
                                range: light.range().unwrap_or(20.0),
                                radius: light.range().unwrap_or(0.0),
                                inner_angle: inner_cone_angle,
                                outer_angle: outer_cone_angle,
                                ..Default::default()
                            },
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                }
            }
        }

        // append other nodes
        for child in gltf_node.children() {
            if let Err(err) = load_node(
                &child,
                parent,
                root_load_context,
                load_context,
                settings,
                node_index_to_entity_map,
                entity_to_skin_index_map,
                active_camera_found,
                &world_transform,
            ) {
                gltf_error = Some(err);
                return;
            }
        }
    });

    if settings.load_meshes {
        if let (Some(mesh), Some(weights)) = (gltf_node.mesh(), morph_weights) {
            let primitive_label = mesh.primitives().next().map(|p| primitive_label(&mesh, &p));
            let first_mesh = primitive_label.map(|label| load_context.get_label_handle(label));
            node.insert(MorphWeights::new(weights, first_mesh)?);
        }
    }

    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Returns the label for the `mesh`.
fn mesh_label(mesh: &gltf::Mesh) -> String {
    format!("Mesh{}", mesh.index())
}

/// Returns the label for the `mesh` and `primitive`.
fn primitive_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!("Mesh{}/Primitive{}", mesh.index(), primitive.index())
}

fn primitive_name(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    let mesh_name = mesh.name().unwrap_or("Mesh");
    if mesh.primitives().len() > 1 {
        format!("{}.{}", mesh_name, primitive.index())
    } else {
        mesh_name.to_string()
    }
}

/// Returns the label for the morph target of `primitive`.
fn morph_targets_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!(
        "Mesh{}/Primitive{}/MorphTargets",
        mesh.index(),
        primitive.index()
    )
}

/// Returns the label for the `material`.
fn material_label(material: &Material, is_scale_inverted: bool) -> String {
    if let Some(index) = material.index() {
        format!(
            "Material{index}{}",
            if is_scale_inverted { " (inverted)" } else { "" }
        )
    } else {
        "MaterialDefault".to_string()
    }
}

/// Returns the label for the `texture`.
fn texture_label(texture: &gltf::Texture) -> String {
    format!("Texture{}", texture.index())
}

fn texture_handle(load_context: &mut LoadContext, texture: &gltf::Texture) -> Handle<Image> {
    match texture.source().source() {
        gltf::image::Source::View { .. } => {
            let label = texture_label(texture);
            load_context.get_label_handle(&label)
        }
        gltf::image::Source::Uri { uri, .. } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            if let Ok(_data_uri) = DataUri::parse(uri) {
                let label = texture_label(texture);
                load_context.get_label_handle(&label)
            } else {
                let parent = load_context.path().parent().unwrap();
                let image_path = parent.join(uri);
                load_context.load(image_path)
            }
        }
    }
}

/// Returns the label for the `node`.
fn node_label(node: &Node) -> String {
    format!("Node{}", node.index())
}

/// Returns the label for the `scene`.
fn scene_label(scene: &gltf::Scene) -> String {
    format!("Scene{}", scene.index())
}

fn skin_label(skin: &gltf::Skin) -> String {
    format!("Skin{}", skin.index())
}

/// Extracts the texture sampler data from the glTF texture.
fn texture_sampler(texture: &gltf::Texture) -> ImageSamplerDescriptor {
    let gltf_sampler = texture.sampler();

    ImageSamplerDescriptor {
        address_mode_u: texture_address_mode(&gltf_sampler.wrap_s()),
        address_mode_v: texture_address_mode(&gltf_sampler.wrap_t()),

        mag_filter: gltf_sampler
            .mag_filter()
            .map(|mf| match mf {
                MagFilter::Nearest => ImageFilterMode::Nearest,
                MagFilter::Linear => ImageFilterMode::Linear,
            })
            .unwrap_or(ImageSamplerDescriptor::default().mag_filter),

        min_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::NearestMipmapNearest
                | MinFilter::NearestMipmapLinear => ImageFilterMode::Nearest,
                MinFilter::Linear
                | MinFilter::LinearMipmapNearest
                | MinFilter::LinearMipmapLinear => ImageFilterMode::Linear,
            })
            .unwrap_or(ImageSamplerDescriptor::default().min_filter),

        mipmap_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::Linear
                | MinFilter::NearestMipmapNearest
                | MinFilter::LinearMipmapNearest => ImageFilterMode::Nearest,
                MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                    ImageFilterMode::Linear
                }
            })
            .unwrap_or(ImageSamplerDescriptor::default().mipmap_filter),

        ..Default::default()
    }
}

/// Maps the texture address mode form glTF to wgpu.
fn texture_address_mode(gltf_address_mode: &WrappingMode) -> ImageAddressMode {
    match gltf_address_mode {
        WrappingMode::ClampToEdge => ImageAddressMode::ClampToEdge,
        WrappingMode::Repeat => ImageAddressMode::Repeat,
        WrappingMode::MirroredRepeat => ImageAddressMode::MirrorRepeat,
    }
}

/// Maps the `primitive_topology` form glTF to `wgpu`.
#[allow(clippy::result_large_err)]
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

fn alpha_mode(material: &Material) -> AlphaMode {
    match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}

/// Loads the raw glTF buffer data for a specific glTF file.
async fn load_buffers(
    gltf: &gltf::Gltf,
    load_context: &mut LoadContext<'_>,
) -> Result<Vec<Vec<u8>>, GltfError> {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                        data_uri.decode()?
                    }
                    Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                    Err(()) => {
                        // TODO: Remove this and add dep
                        let buffer_path = load_context.path().parent().unwrap().join(uri);
                        load_context.read_asset_bytes(buffer_path).await?
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
    asset_path: &Path,
) -> Vec<(String, GltfNode)> {
    let mut has_errored = false;
    let mut empty_children = VecDeque::new();
    let mut parents = vec![None; nodes_intermediate.len()];
    let mut unprocessed_nodes = nodes_intermediate
        .into_iter()
        .enumerate()
        .map(|(i, (label, node, children))| {
            for child in &children {
                if let Some(parent) = parents.get_mut(*child) {
                    *parent = Some(i);
                } else if !has_errored {
                    has_errored = true;
                    warn!("Unexpected child in GLTF Mesh {}", child);
                }
            }
            let children = children.into_iter().collect::<HashSet<_>>();
            if children.is_empty() {
                empty_children.push_back(i);
            }
            (i, (label, node, children))
        })
        .collect::<HashMap<_, _>>();
    let mut nodes = std::collections::HashMap::<usize, (String, GltfNode)>::new();
    while let Some(index) = empty_children.pop_front() {
        let (label, node, children) = unprocessed_nodes.remove(&index).unwrap();
        assert!(children.is_empty());
        nodes.insert(index, (label, node));
        if let Some(parent_index) = parents[index] {
            let (_, parent_node, parent_children) =
                unprocessed_nodes.get_mut(&parent_index).unwrap();

            assert!(parent_children.remove(&index));
            if let Some((_, child_node)) = nodes.get(&index) {
                parent_node.children.push(child_node.clone());
            }
            if parent_children.is_empty() {
                empty_children.push_back(parent_index);
            }
        }
    }
    if !unprocessed_nodes.is_empty() {
        warn!("GLTF model must be a tree: {:?}", asset_path);
    }
    let mut nodes_to_sort = nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect()
}

enum ImageOrPath {
    Image {
        image: Image,
        label: String,
    },
    Path {
        path: PathBuf,
        is_srgb: bool,
        sampler_descriptor: ImageSamplerDescriptor,
    },
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
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

pub(super) struct PrimitiveMorphAttributesIter<'s>(
    pub  (
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
    ),
);
impl<'s> Iterator for PrimitiveMorphAttributesIter<'s> {
    type Item = MorphAttributes;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.0 .0.as_mut().and_then(|p| p.next());
        let normal = self.0 .1.as_mut().and_then(|n| n.next());
        let tangent = self.0 .2.as_mut().and_then(|t| t.next());
        if position.is_none() && normal.is_none() && tangent.is_none() {
            return None;
        }

        Some(MorphAttributes {
            position: position.map(|p| p.into()).unwrap_or(Vec3::ZERO),
            normal: normal.map(|n| n.into()).unwrap_or(Vec3::ZERO),
            tangent: tangent.map(|t| t.into()).unwrap_or(Vec3::ZERO),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MorphTargetNames {
    pub target_names: Vec<String>,
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::resolve_node_hierarchy;
    use crate::GltfNode;

    impl GltfNode {
        fn empty() -> Self {
            GltfNode {
                children: vec![],
                mesh: None,
                transform: bevy_transform::prelude::Transform::IDENTITY,
                extras: None,
            }
        }
    }
    #[test]
    fn node_hierarchy_single_node() {
        let result = resolve_node_hierarchy(
            vec![("l1".to_string(), GltfNode::empty(), vec![])],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_no_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_simple_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 1);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![2]),
                ("l3".to_string(), GltfNode::empty(), vec![3, 4, 5]),
                ("l4".to_string(), GltfNode::empty(), vec![6]),
                ("l5".to_string(), GltfNode::empty(), vec![]),
                ("l6".to_string(), GltfNode::empty(), vec![]),
                ("l7".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

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
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![0]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn node_hierarchy_missing_node() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![2]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l2");
        assert_eq!(result[0].1.children.len(), 0);
    }
}
