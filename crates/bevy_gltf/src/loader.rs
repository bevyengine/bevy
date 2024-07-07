use crate::{
    vertex_attributes::convert_attribute, Gltf, GltfAssetLabel, GltfExtras, GltfMaterialExtras,
    GltfMeshExtras, GltfNode, GltfSceneExtras,
};

#[cfg(feature = "bevy_animation")]
use bevy_animation::{AnimationTarget, AnimationTargetId};
use bevy_asset::{
    io::Reader, AssetLoadError, AssetLoader, AsyncReadExt, Handle, LoadContext, ReadAssetBytesError,
};
use bevy_color::{Color, LinearRgba};
use bevy_core::Name;
use bevy_core_pipeline::prelude::Camera3dBundle;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::{entity::Entity, world::World};
use bevy_hierarchy::{BuildWorldChildren, WorldChildBuilder};
use bevy_math::{Affine2, Mat4, Vec3};
use bevy_pbr::{
    DirectionalLight, DirectionalLightBundle, PbrBundle, PointLight, PointLightBundle, SpotLight,
    SpotLightBundle, StandardMaterial, UvChannel, MAX_JOINTS,
};
use bevy_render::{
    alpha::AlphaMode,
    camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode},
    mesh::{
        morph::{MeshMorphWeights, MorphAttributes, MorphTargetImage, MorphWeights},
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        Indices, Mesh, MeshVertexAttribute, VertexAttributeValues,
    },
    prelude::SpatialBundle,
    primitives::Aabb,
    render_asset::RenderAssetUsages,
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
use bevy_utils::tracing::{error, info_span, warn};
use bevy_utils::{HashMap, HashSet};
use gltf::image::Source;
use gltf::{
    accessor::Iter,
    mesh::{util::ReadIndices, Mode},
    texture::{Info, MagFilter, MinFilter, TextureTransform, WrappingMode},
    Material, Node, Primitive, Semantic,
};
use gltf::{json, Document};
use serde::{Deserialize, Serialize};
use serde_json::{value, Value};
#[cfg(feature = "bevy_animation")]
use smallvec::SmallVec;
use std::io::Error;
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
    pub custom_vertex_attributes: HashMap<Box<str>, MeshVertexAttribute>,
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
    /// If empty, the gltf mesh nodes will be skipped.
    ///
    /// Otherwise, nodes will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_meshes: RenderAssetUsages,
    /// If empty, the gltf materials will be skipped.
    ///
    /// Otherwise, materials will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_materials: RenderAssetUsages,
    /// If true, the loader will spawn cameras for gltf camera nodes.
    pub load_cameras: bool,
    /// If true, the loader will spawn lights for gltf light nodes.
    pub load_lights: bool,
    /// If true, the loader will include the root of the gltf root node.
    pub include_source: bool,
}

impl Default for GltfLoaderSettings {
    fn default() -> Self {
        Self {
            load_meshes: RenderAssetUsages::default(),
            load_materials: RenderAssetUsages::default(),
            load_cameras: true,
            load_lights: true,
            include_source: false,
        }
    }
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = GltfLoaderSettings;
    type Error = GltfError;
    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        settings: &'a GltfLoaderSettings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Gltf, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        load_gltf(self, &bytes, load_context, settings).await
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
    let file_name = load_context
        .asset_path()
        .path()
        .to_str()
        .ok_or(GltfError::Gltf(gltf::Error::Io(Error::new(
            std::io::ErrorKind::InvalidInput,
            "Gltf file name invalid",
        ))))?
        .to_string();
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
        if let Some(texture_index) = material_extension_texture_index(
            &material,
            "KHR_materials_anisotropy",
            "anisotropyTexture",
        ) {
            linear_textures.insert(texture_index);
        }

        // None of the clearcoat maps should be loaded as sRGB.
        #[cfg(feature = "pbr_multi_layer_material_textures")]
        for texture_field_name in [
            "clearcoatTexture",
            "clearcoatRoughnessTexture",
            "clearcoatNormalTexture",
        ] {
            if let Some(texture_index) = material_extension_texture_index(
                &material,
                "KHR_materials_clearcoat",
                texture_field_name,
            ) {
                linear_textures.insert(texture_index);
            }
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
                    animation_roots.insert(*root_index);
                    animation_clip.add_curve_to_target(
                        AnimationTargetId::from_names(path.iter()),
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
            let handle = load_context.add_labeled_asset(
                GltfAssetLabel::Animation(animation.index()).to_string(),
                animation_clip,
            );
            if let Some(name) = animation.name() {
                named_animations.insert(name.into(), handle.clone());
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
            ImageOrPath::Image { label, image } => {
                load_context.add_labeled_asset(label.to_string(), image)
            }
            ImageOrPath::Path {
                path,
                is_srgb,
                sampler_descriptor,
            } => load_context
                .loader()
                .with_settings(move |settings: &mut ImageLoaderSettings| {
                    settings.is_srgb = is_srgb;
                    settings.sampler = ImageSampler::Descriptor(sampler_descriptor.clone());
                })
                .load(path),
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
                settings.load_materials,
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
                            settings.load_materials,
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
    // Only include materials in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_materials flag
    if !settings.load_materials.is_empty() {
        // NOTE: materials must be loaded after textures because image load() calls will happen before load_with_settings, preventing is_srgb from being set properly
        for material in gltf.materials() {
            let handle = load_material(&material, load_context, &gltf.document, false);
            if let Some(name) = material.name() {
                named_materials.insert(name.into(), handle.clone());
            }
            materials.push(handle);
        }
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
            let primitive_label = GltfAssetLabel::Primitive {
                mesh: gltf_mesh.index(),
                primitive: primitive.index(),
            };
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology, settings.load_meshes);

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
                mesh.insert_indices(match indices {
                    ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
                    ReadIndices::U16(is) => Indices::U16(is.collect()),
                    ReadIndices::U32(is) => Indices::U32(is.collect()),
                });
            };

            {
                let morph_target_reader = reader.read_morph_targets();
                if morph_target_reader.len() != 0 {
                    let morph_targets_label = GltfAssetLabel::MorphTarget {
                        mesh: gltf_mesh.index(),
                        primitive: primitive.index(),
                    };
                    let morph_target_image = MorphTargetImage::new(
                        morph_target_reader.map(PrimitiveMorphAttributesIter),
                        mesh.count_vertices(),
                        RenderAssetUsages::default(),
                    )?;
                    let handle = load_context
                        .add_labeled_asset(morph_targets_label.to_string(), morph_target_image.0);

                    mesh.set_morph_targets(handle);
                    let extras = gltf_mesh.extras().as_ref();
                    if let Some(names) = extras.and_then(|extras| {
                        serde_json::from_str::<MorphTargetNames>(extras.get()).ok()
                    }) {
                        mesh.set_morph_target_names(names.target_names);
                    }
                }
            }

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
                && matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
            {
                bevy_utils::tracing::debug!(
                    "Automatically calculating missing vertex normals for geometry."
                );
                let vertex_count_before = mesh.count_vertices();
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let vertex_count_after = mesh.count_vertices();
                if vertex_count_before != vertex_count_after {
                    bevy_utils::tracing::debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
                } else {
                    bevy_utils::tracing::debug!(
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
                && material_needs_tangents(&primitive.material())
            {
                bevy_utils::tracing::debug!(
                    "Missing vertex tangents for {}, computing them using the mikktspace algorithm. Consider using a tool such as Blender to pre-compute the tangents.", file_name
                );

                let generate_tangents_span = info_span!("generate_tangents", name = file_name);

                generate_tangents_span.in_scope(|| {
                    if let Err(err) = mesh.generate_tangents() {
                        warn!(
                        "Failed to generate vertex tangents using the mikktspace algorithm: {:?}",
                        err
                    );
                    }
                });
            }

            let mesh_handle = load_context.add_labeled_asset(primitive_label.to_string(), mesh);
            primitives.push(super::GltfPrimitive::new(
                &gltf_mesh,
                &primitive,
                mesh_handle,
                primitive
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned()),
                get_gltf_extras(primitive.extras()),
                get_gltf_extras(primitive.material().extras()),
            ));
        }

        let mesh =
            super::GltfMesh::new(&gltf_mesh, primitives, get_gltf_extras(gltf_mesh.extras()));

        let handle = load_context.add_labeled_asset(mesh.asset_label.to_string(), mesh);
        if let Some(name) = gltf_mesh.name() {
            named_meshes.insert(name.into(), handle.clone());
        }
        meshes.push(handle);
    }

    let mut nodes_intermediate = vec![];
    let mut named_nodes_intermediate = HashMap::default();
    for node in gltf.nodes() {
        nodes_intermediate.push((
            GltfNode::new(
                &node,
                vec![],
                node.mesh()
                    .map(|mesh| mesh.index())
                    .and_then(|i: usize| meshes.get(i).cloned()),
                node_transform(&node),
                get_gltf_extras(node.extras()),
            ),
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
        .map(|node| load_context.add_labeled_asset(node.asset_label.to_string(), node))
        .collect::<Vec<Handle<GltfNode>>>();
    let named_nodes = named_nodes_intermediate
        .into_iter()
        .filter_map(|(name, index)| nodes.get(index).map(|handle| (name.into(), handle.clone())))
        .collect();

    let skinned_mesh_inverse_bindposes: Vec<_> = gltf
        .skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let local_to_bone_bind_matrices: Vec<Mat4> = reader
                .read_inverse_bind_matrices()
                .unwrap()
                .map(|mat| Mat4::from_cols_array_2d(&mat))
                .collect();

            load_context.add_labeled_asset(
                skin_label(&gltf_skin),
                SkinnedMeshInverseBindposes::from(local_to_bone_bind_matrices),
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
        let mut entity_to_skin_index_map = EntityHashMap::default();
        let mut scene_load_context = load_context.begin_labeled_asset();

        let world_root_id = world
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
                        #[cfg(feature = "bevy_animation")]
                        &animation_roots,
                        #[cfg(feature = "bevy_animation")]
                        None,
                        &gltf.document,
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            })
            .id();

        if let Some(extras) = scene.extras().as_ref() {
            world.entity_mut(world_root_id).insert(GltfSceneExtras {
                value: extras.get().to_string(),
            });
        }

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
            named_scenes.insert(name.into(), scene_handle.clone());
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
        source: if settings.include_source {
            Some(gltf)
        } else {
            None
        },
    })
}

fn get_gltf_extras(extras: &gltf::json::Extras) -> Option<GltfExtras> {
    extras.as_ref().map(|extras| GltfExtras {
        value: extras.get().to_string(),
    })
}

/// Calculate the transform of gLTF node.
///
/// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
/// on [`Node::transform()`] directly because it uses optimized glam types and
/// if `libm` feature of `bevy_math` crate is enabled also handles cross
/// platform determinism properly.
fn node_transform(node: &Node) -> Transform {
    match node.transform() {
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
    }
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
    render_asset_usages: RenderAssetUsages,
) -> Result<ImageOrPath, GltfError> {
    let is_srgb = !linear_textures.contains(&gltf_texture.index());
    let sampler_descriptor = texture_sampler(&gltf_texture);
    #[cfg(all(debug_assertions, feature = "dds"))]
    let name = gltf_texture
        .name()
        .map_or("Unknown GLTF Texture".to_string(), |s| s.to_string());
    match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset();
            let end = view.offset() + view.length();
            let buffer = &buffer_data[view.buffer().index()][start..end];
            let image = Image::from_buffer(
                #[cfg(all(debug_assertions, feature = "dds"))]
                name,
                buffer,
                ImageType::MimeType(mime_type),
                supported_compressed_formats,
                is_srgb,
                ImageSampler::Descriptor(sampler_descriptor),
                render_asset_usages,
            )?;
            Ok(ImageOrPath::Image {
                image,
                label: GltfAssetLabel::Texture(gltf_texture.index()),
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
                        #[cfg(all(debug_assertions, feature = "dds"))]
                        name,
                        &bytes,
                        mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                        supported_compressed_formats,
                        is_srgb,
                        ImageSampler::Descriptor(sampler_descriptor),
                        render_asset_usages,
                    )?,
                    label: GltfAssetLabel::Texture(gltf_texture.index()),
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
    document: &Document,
    is_scale_inverted: bool,
) -> Handle<StandardMaterial> {
    let material_label = material_label(material, is_scale_inverted);
    load_context.labeled_asset_scope(material_label, |load_context| {
        let pbr = material.pbr_metallic_roughness();

        // TODO: handle missing label handle errors here?
        let color = pbr.base_color_factor();
        let base_color_channel = pbr
            .base_color_texture()
            .map(|info| get_uv_channel(material, "base color", info.tex_coord()))
            .unwrap_or_default();
        let base_color_texture = pbr
            .base_color_texture()
            .map(|info| texture_handle(load_context, &info.texture()));

        let uv_transform = pbr
            .base_color_texture()
            .and_then(|info| {
                info.texture_transform()
                    .map(convert_texture_transform_to_affine2)
            })
            .unwrap_or_default();

        let normal_map_channel = material
            .normal_texture()
            .map(|info| get_uv_channel(material, "normal map", info.tex_coord()))
            .unwrap_or_default();
        let normal_map_texture: Option<Handle<Image>> =
            material.normal_texture().map(|normal_texture| {
                // TODO: handle normal_texture.scale
                texture_handle(load_context, &normal_texture.texture())
            });

        let metallic_roughness_channel = pbr
            .metallic_roughness_texture()
            .map(|info| get_uv_channel(material, "metallic/roughness", info.tex_coord()))
            .unwrap_or_default();
        let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
            warn_on_differing_texture_transforms(
                material,
                &info,
                uv_transform,
                "metallic/roughness",
            );
            texture_handle(load_context, &info.texture())
        });

        let occlusion_channel = material
            .occlusion_texture()
            .map(|info| get_uv_channel(material, "occlusion", info.tex_coord()))
            .unwrap_or_default();
        let occlusion_texture = material.occlusion_texture().map(|occlusion_texture| {
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            texture_handle(load_context, &occlusion_texture.texture())
        });

        let emissive = material.emissive_factor();
        let emissive_channel = material
            .emissive_texture()
            .map(|info| get_uv_channel(material, "emissive", info.tex_coord()))
            .unwrap_or_default();
        let emissive_texture = material.emissive_texture().map(|info| {
            // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
            warn_on_differing_texture_transforms(material, &info, uv_transform, "emissive");
            texture_handle(load_context, &info.texture())
        });

        #[cfg(feature = "pbr_transmission_textures")]
        let (specular_transmission, specular_transmission_channel, specular_transmission_texture) =
            material
                .transmission()
                .map_or((0.0, UvChannel::Uv0, None), |transmission| {
                    let specular_transmission_channel = transmission
                        .transmission_texture()
                        .map(|info| {
                            get_uv_channel(material, "specular/transmission", info.tex_coord())
                        })
                        .unwrap_or_default();
                    let transmission_texture: Option<Handle<Image>> = transmission
                        .transmission_texture()
                        .map(|transmission_texture| {
                            texture_handle(load_context, &transmission_texture.texture())
                        });

                    (
                        transmission.transmission_factor(),
                        specular_transmission_channel,
                        transmission_texture,
                    )
                });

        #[cfg(not(feature = "pbr_transmission_textures"))]
        let specular_transmission = material
            .transmission()
            .map_or(0.0, |transmission| transmission.transmission_factor());

        #[cfg(feature = "pbr_transmission_textures")]
        let (
            thickness,
            thickness_channel,
            thickness_texture,
            attenuation_distance,
            attenuation_color,
        ) = material.volume().map_or(
            (0.0, UvChannel::Uv0, None, f32::INFINITY, [1.0, 1.0, 1.0]),
            |volume| {
                let thickness_channel = volume
                    .thickness_texture()
                    .map(|info| get_uv_channel(material, "thickness", info.tex_coord()))
                    .unwrap_or_default();
                let thickness_texture: Option<Handle<Image>> =
                    volume.thickness_texture().map(|thickness_texture| {
                        texture_handle(load_context, &thickness_texture.texture())
                    });

                (
                    volume.thickness_factor(),
                    thickness_channel,
                    thickness_texture,
                    volume.attenuation_distance(),
                    volume.attenuation_color(),
                )
            },
        );

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

        // Parse the `KHR_materials_clearcoat` extension data if necessary.
        let clearcoat =
            ClearcoatExtension::parse(load_context, document, material).unwrap_or_default();

        // Parse the `KHR_materials_anisotropy` extension data if necessary.
        let anisotropy =
            AnisotropyExtension::parse(load_context, document, material).unwrap_or_default();

        // We need to operate in the Linear color space and be willing to exceed 1.0 in our channels
        let base_emissive = LinearRgba::rgb(emissive[0], emissive[1], emissive[2]);
        let emissive = base_emissive * material.emissive_strength().unwrap_or(1.0);

        StandardMaterial {
            base_color: Color::linear_rgba(color[0], color[1], color[2], color[3]),
            base_color_channel,
            base_color_texture,
            perceptual_roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_channel,
            metallic_roughness_texture,
            normal_map_channel,
            normal_map_texture,
            double_sided: material.double_sided(),
            cull_mode: if material.double_sided() {
                None
            } else if is_scale_inverted {
                Some(Face::Front)
            } else {
                Some(Face::Back)
            },
            occlusion_channel,
            occlusion_texture,
            emissive,
            emissive_channel,
            emissive_texture,
            specular_transmission,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_channel,
            #[cfg(feature = "pbr_transmission_textures")]
            specular_transmission_texture,
            thickness,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_channel,
            #[cfg(feature = "pbr_transmission_textures")]
            thickness_texture,
            ior,
            attenuation_distance,
            attenuation_color: Color::linear_rgb(
                attenuation_color[0],
                attenuation_color[1],
                attenuation_color[2],
            ),
            unlit: material.unlit(),
            alpha_mode: alpha_mode(material),
            uv_transform,
            clearcoat: clearcoat.clearcoat_factor.unwrap_or_default() as f32,
            clearcoat_perceptual_roughness: clearcoat.clearcoat_roughness_factor.unwrap_or_default()
                as f32,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: clearcoat.clearcoat_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture: clearcoat.clearcoat_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: clearcoat.clearcoat_roughness_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture: clearcoat.clearcoat_roughness_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: clearcoat.clearcoat_normal_channel,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture: clearcoat.clearcoat_normal_texture,
            anisotropy_strength: anisotropy.anisotropy_strength.unwrap_or_default() as f32,
            anisotropy_rotation: anisotropy.anisotropy_rotation.unwrap_or_default() as f32,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: anisotropy.anisotropy_channel,
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture: anisotropy.anisotropy_texture,
            ..Default::default()
        }
    })
}

fn get_uv_channel(material: &Material, texture_kind: &str, tex_coord: u32) -> UvChannel {
    match tex_coord {
        0 => UvChannel::Uv0,
        1 => UvChannel::Uv1,
        _ => {
            let material_name = material
                .name()
                .map(|n| format!("the material \"{n}\""))
                .unwrap_or_else(|| "an unnamed material".to_string());
            let material_index = material
                .index()
                .map(|i| format!("index {i}"))
                .unwrap_or_else(|| "default".to_string());
            warn!(
                "Only 2 UV Channels are supported, but {material_name} ({material_index}) \
                has the TEXCOORD attribute {} on texture kind {texture_kind}, which will fallback to 0.",
                tex_coord,
            );
            UvChannel::Uv0
        }
    }
}

fn convert_texture_transform_to_affine2(texture_transform: TextureTransform) -> Affine2 {
    Affine2::from_scale_angle_translation(
        texture_transform.scale().into(),
        -texture_transform.rotation(),
        texture_transform.offset().into(),
    )
}

fn warn_on_differing_texture_transforms(
    material: &Material,
    info: &Info,
    texture_transform: Affine2,
    texture_kind: &str,
) {
    let has_differing_texture_transform = info
        .texture_transform()
        .map(convert_texture_transform_to_affine2)
        .is_some_and(|t| t != texture_transform);
    if has_differing_texture_transform {
        let material_name = material
            .name()
            .map(|n| format!("the material \"{n}\""))
            .unwrap_or_else(|| "an unnamed material".to_string());
        let texture_name = info
            .texture()
            .name()
            .map(|n| format!("its {texture_kind} texture \"{n}\""))
            .unwrap_or_else(|| format!("its unnamed {texture_kind} texture"));
        let material_index = material
            .index()
            .map(|i| format!("index {i}"))
            .unwrap_or_else(|| "default".to_string());
        warn!(
            "Only texture transforms on base color textures are supported, but {material_name} ({material_index}) \
            has a texture transform on {texture_name} (index {}), which will be ignored.", info.texture().index()
        );
    }
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
    entity_to_skin_index_map: &mut EntityHashMap<usize>,
    active_camera_found: &mut bool,
    parent_transform: &Transform,
    #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    #[cfg(feature = "bevy_animation")] mut animation_context: Option<AnimationContext>,
    document: &Document,
) -> Result<(), GltfError> {
    let mut gltf_error = None;
    let transform = node_transform(gltf_node);
    let world_transform = *parent_transform * transform;
    // according to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#instantiation,
    // if the determinant of the transform is negative we must invert the winding order of
    // triangles in meshes on the node.
    // instead we equivalently test if the global scale is inverted by checking if the number
    // of negative scale factors is odd. if so we will assign a copy of the material with face
    // culling inverted, rather than modifying the mesh data directly.
    let is_scale_inverted = world_transform.scale.is_negative_bitmask().count_ones() & 1 == 1;
    let mut node = world_builder.spawn(SpatialBundle::from(transform));

    let name = node_name(gltf_node);
    node.insert(name.clone());

    #[cfg(feature = "bevy_animation")]
    if animation_context.is_none() && animation_roots.contains(&gltf_node.index()) {
        // This is an animation root. Make a new animation context.
        animation_context = Some(AnimationContext {
            root: node.id(),
            path: SmallVec::new(),
        });
    }

    #[cfg(feature = "bevy_animation")]
    if let Some(ref mut animation_context) = animation_context {
        animation_context.path.push(name);

        node.insert(AnimationTarget {
            id: AnimationTargetId::from_names(animation_context.path.iter()),
            player: animation_context.root,
        });
    }

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
        // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
        if !settings.load_meshes.is_empty() {
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
                        load_material(&material, load_context, document, is_scale_inverted);
                    }

                    let primitive_label = GltfAssetLabel::Primitive {
                        mesh: mesh.index(),
                        primitive: primitive.index(),
                    };
                    let bounds = primitive.bounding_box();

                    let mut mesh_entity = parent.spawn(PbrBundle {
                        // TODO: handle missing label handle errors here?
                        mesh: load_context.get_label_handle(primitive_label.to_string()),
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

                    if let Some(extras) = mesh.extras() {
                        mesh_entity.insert(GltfMeshExtras {
                            value: extras.get().to_string(),
                        });
                    }

                    if let Some(extras) = material.extras() {
                        mesh_entity.insert(GltfMaterialExtras {
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
                                color: Color::srgb_from_array(light.color()),
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
                                color: Color::srgb_from_array(light.color()),
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
                                color: Color::srgb_from_array(light.color()),
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
                #[cfg(feature = "bevy_animation")]
                animation_roots,
                #[cfg(feature = "bevy_animation")]
                animation_context.clone(),
                document,
            ) {
                gltf_error = Some(err);
                return;
            }
        }
    });

    // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
    if !settings.load_meshes.is_empty() {
        if let (Some(mesh), Some(weights)) = (gltf_node.mesh(), morph_weights) {
            let primitive_label = mesh.primitives().next().map(|p| GltfAssetLabel::Primitive {
                mesh: mesh.index(),
                primitive: p.index(),
            });
            let first_mesh =
                primitive_label.map(|label| load_context.get_label_handle(label.to_string()));
            node.insert(MorphWeights::new(weights, first_mesh)?);
        }
    }

    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

fn primitive_name(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    let mesh_name = mesh.name().unwrap_or("Mesh");
    if mesh.primitives().len() > 1 {
        format!("{}.{}", mesh_name, primitive.index())
    } else {
        mesh_name.to_string()
    }
}

/// Returns the label for the `material`.
fn material_label(material: &Material, is_scale_inverted: bool) -> String {
    if let Some(index) = material.index() {
        GltfAssetLabel::Material {
            index,
            is_scale_inverted,
        }
        .to_string()
    } else {
        GltfAssetLabel::DefaultMaterial.to_string()
    }
}

fn texture_handle(load_context: &mut LoadContext, texture: &gltf::Texture) -> Handle<Image> {
    match texture.source().source() {
        Source::View { .. } => {
            load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
        }
        Source::Uri { uri, .. } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            if let Ok(_data_uri) = DataUri::parse(uri) {
                load_context.get_label_handle(GltfAssetLabel::Texture(texture.index()).to_string())
            } else {
                let parent = load_context.path().parent().unwrap();
                let image_path = parent.join(uri);
                load_context.load(image_path)
            }
        }
    }
}

/// Given a [`json::texture::Info`], returns the handle of the texture that this
/// refers to.
///
/// This is a low-level function only used when the `gltf` crate has no support
/// for an extension, forcing us to parse its texture references manually.
#[allow(dead_code)]
fn texture_handle_from_info(
    load_context: &mut LoadContext,
    document: &Document,
    texture_info: &json::texture::Info,
) -> Handle<Image> {
    let texture = document
        .textures()
        .nth(texture_info.index.value())
        .expect("Texture info references a nonexistent texture");
    texture_handle(load_context, &texture)
}

/// Returns the label for the `scene`.
fn scene_label(scene: &gltf::Scene) -> String {
    GltfAssetLabel::Scene(scene.index()).to_string()
}

fn skin_label(skin: &gltf::Skin) -> String {
    GltfAssetLabel::Skin(skin.index()).to_string()
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
    nodes_intermediate: Vec<(GltfNode, Vec<usize>)>,
    asset_path: &Path,
) -> Vec<GltfNode> {
    let mut has_errored = false;
    let mut empty_children = VecDeque::new();
    let mut parents = vec![None; nodes_intermediate.len()];
    let mut unprocessed_nodes = nodes_intermediate
        .into_iter()
        .enumerate()
        .map(|(i, (node, children))| {
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
            (i, (node, children))
        })
        .collect::<HashMap<_, _>>();
    let mut nodes = std::collections::HashMap::<usize, GltfNode>::new();
    while let Some(index) = empty_children.pop_front() {
        let (node, children) = unprocessed_nodes.remove(&index).unwrap();
        assert!(children.is_empty());
        nodes.insert(index, node);
        if let Some(parent_index) = parents[index] {
            let (parent_node, parent_children) = unprocessed_nodes.get_mut(&parent_index).unwrap();

            assert!(parent_children.remove(&index));
            if let Some(child_node) = nodes.get(&index) {
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
        label: GltfAssetLabel,
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

// A helper structure for `load_node` that contains information about the
// nearest ancestor animation root.
#[cfg(feature = "bevy_animation")]
#[derive(Clone)]
struct AnimationContext {
    // The nearest ancestor animation root.
    root: Entity,
    // The path to the animation root. This is used for constructing the
    // animation target UUIDs.
    path: SmallVec<[Name; 8]>,
}

/// Parsed data from the `KHR_materials_clearcoat` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_clearcoat/README.md>
#[derive(Default)]
struct ClearcoatExtension {
    clearcoat_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_texture: Option<Handle<Image>>,
    clearcoat_roughness_factor: Option<f64>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_roughness_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_roughness_texture: Option<Handle<Image>>,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_normal_channel: UvChannel,
    #[cfg(feature = "pbr_multi_layer_material_textures")]
    clearcoat_normal_texture: Option<Handle<Image>>,
}

impl ClearcoatExtension {
    #[allow(unused_variables)]
    fn parse(
        load_context: &mut LoadContext,
        document: &Document,
        material: &Material,
    ) -> Option<ClearcoatExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_clearcoat")?
            .as_object()?;

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_channel, clearcoat_texture) = extension
            .get("clearcoatTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    get_uv_channel(material, "clearcoat", json_info.tex_coord),
                    texture_handle_from_info(load_context, document, &json_info),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_roughness_channel, clearcoat_roughness_texture) = extension
            .get("clearcoatRoughnessTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    get_uv_channel(material, "clearcoat roughness", json_info.tex_coord),
                    texture_handle_from_info(load_context, document, &json_info),
                )
            })
            .unzip();

        #[cfg(feature = "pbr_multi_layer_material_textures")]
        let (clearcoat_normal_channel, clearcoat_normal_texture) = extension
            .get("clearcoatNormalTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    get_uv_channel(material, "clearcoat normal", json_info.tex_coord),
                    texture_handle_from_info(load_context, document, &json_info),
                )
            })
            .unzip();

        Some(ClearcoatExtension {
            clearcoat_factor: extension.get("clearcoatFactor").and_then(Value::as_f64),
            clearcoat_roughness_factor: extension
                .get("clearcoatRoughnessFactor")
                .and_then(Value::as_f64),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_channel: clearcoat_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_channel: clearcoat_roughness_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_roughness_texture,
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_channel: clearcoat_normal_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            clearcoat_normal_texture,
        })
    }
}

/// Parsed data from the `KHR_materials_anisotropy` extension.
///
/// See the specification:
/// <https://github.com/KhronosGroup/glTF/blob/main/extensions/2.0/Khronos/KHR_materials_anisotropy/README.md>
#[derive(Default)]
struct AnisotropyExtension {
    anisotropy_strength: Option<f64>,
    anisotropy_rotation: Option<f64>,
    #[cfg(feature = "pbr_anisotropy_texture")]
    anisotropy_channel: UvChannel,
    #[cfg(feature = "pbr_anisotropy_texture")]
    anisotropy_texture: Option<Handle<Image>>,
}

impl AnisotropyExtension {
    #[allow(unused_variables)]
    fn parse(
        load_context: &mut LoadContext,
        document: &Document,
        material: &Material,
    ) -> Option<AnisotropyExtension> {
        let extension = material
            .extensions()?
            .get("KHR_materials_anisotropy")?
            .as_object()?;

        #[cfg(feature = "pbr_anisotropy_texture")]
        let (anisotropy_channel, anisotropy_texture) = extension
            .get("anisotropyTexture")
            .and_then(|value| value::from_value::<json::texture::Info>(value.clone()).ok())
            .map(|json_info| {
                (
                    get_uv_channel(material, "anisotropy", json_info.tex_coord),
                    texture_handle_from_info(load_context, document, &json_info),
                )
            })
            .unzip();

        Some(AnisotropyExtension {
            anisotropy_strength: extension.get("anisotropyStrength").and_then(Value::as_f64),
            anisotropy_rotation: extension.get("anisotropyRotation").and_then(Value::as_f64),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_channel: anisotropy_channel.unwrap_or_default(),
            #[cfg(feature = "pbr_anisotropy_texture")]
            anisotropy_texture,
        })
    }
}

/// Returns the index (within the `textures` array) of the texture with the
/// given field name in the data for the material extension with the given name,
/// if there is one.
fn material_extension_texture_index(
    material: &Material,
    extension_name: &str,
    texture_field_name: &str,
) -> Option<usize> {
    Some(
        value::from_value::<json::texture::Info>(
            material
                .extensions()?
                .get(extension_name)?
                .as_object()?
                .get(texture_field_name)?
                .clone(),
        )
        .ok()?
        .index
        .value(),
    )
}

/// Returns true if the material needs mesh tangents in order to be successfully
/// rendered.
///
/// We generate them if this function returns true.
fn material_needs_tangents(material: &Material) -> bool {
    if material.normal_texture().is_some() {
        return true;
    }

    #[cfg(feature = "pbr_multi_layer_material_textures")]
    if material_extension_texture_index(
        material,
        "KHR_materials_clearcoat",
        "clearcoatNormalTexture",
    )
    .is_some()
    {
        return true;
    }

    false
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::resolve_node_hierarchy;
    use crate::GltfNode;

    impl GltfNode {
        fn with_generated_name(index: usize) -> Self {
            GltfNode {
                index,
                asset_label: crate::GltfAssetLabel::Node(index),
                name: format!("l{}", index),
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
            vec![(GltfNode::with_generated_name(1), vec![])],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_no_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                (GltfNode::with_generated_name(1), vec![]),
                (GltfNode::with_generated_name(2), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 0);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_simple_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                (GltfNode::with_generated_name(1), vec![1]),
                (GltfNode::with_generated_name(2), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                (GltfNode::with_generated_name(1), vec![1]),
                (GltfNode::with_generated_name(2), vec![2]),
                (GltfNode::with_generated_name(3), vec![3, 4, 5]),
                (GltfNode::with_generated_name(4), vec![6]),
                (GltfNode::with_generated_name(5), vec![]),
                (GltfNode::with_generated_name(6), vec![]),
                (GltfNode::with_generated_name(7), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 7);
        assert_eq!(result[0].name, "l1");
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[1].name, "l2");
        assert_eq!(result[1].children.len(), 1);
        assert_eq!(result[2].name, "l3");
        assert_eq!(result[2].children.len(), 3);
        assert_eq!(result[3].name, "l4");
        assert_eq!(result[3].children.len(), 1);
        assert_eq!(result[4].name, "l5");
        assert_eq!(result[4].children.len(), 0);
        assert_eq!(result[5].name, "l6");
        assert_eq!(result[5].children.len(), 0);
        assert_eq!(result[6].name, "l7");
        assert_eq!(result[6].children.len(), 0);
    }

    #[test]
    fn node_hierarchy_cyclic() {
        let result = resolve_node_hierarchy(
            vec![
                (GltfNode::with_generated_name(1), vec![1]),
                (GltfNode::with_generated_name(2), vec![0]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn node_hierarchy_missing_node() {
        let result = resolve_node_hierarchy(
            vec![
                (GltfNode::with_generated_name(1), vec![2]),
                (GltfNode::with_generated_name(2), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "l2");
        assert_eq!(result[0].children.len(), 0);
    }
}
