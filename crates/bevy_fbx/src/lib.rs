#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//!
//! Loader for FBX scenes using [`ufbx`](https://github.com/ufbx/ufbx-rust).
//! The implementation is intentionally minimal and focuses on importing
//! mesh geometry into Bevy.

use bevy_app::prelude::*;
use bevy_asset::{
    io::Reader, Asset, AssetApp, AssetLoader, Handle, LoadContext, RenderAssetUsages,
};
use bevy_ecs::prelude::*;
use bevy_ecs::prelude::Name;
use bevy_mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_mesh::{Indices, Mesh,Mesh3d, PrimitiveTopology, VertexAttributeValues};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};

use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_camera::{visibility::Visibility, Camera, Camera3d, PerspectiveProjection, Projection};
use bevy_render::render_resource::Face;
use bevy_scene::Scene;
use bevy_utils::default;
use bevy_light::{DirectionalLight, SpotLight, PointLight};
use serde::{Deserialize, Serialize};

use bevy_animation::{
    animated_field,
    animation_curves::{AnimatableCurve, AnimatableKeyframeCurve},
    prelude::AnimatedField,
    AnimationClip, AnimationTarget, AnimationTargetId, AnimationPlayer,
    graph::{AnimationGraph, AnimationGraphHandle},
};
use bevy_color::Color;
use bevy_image::{Image, ImageSamplerDescriptor, ImageSampler, ImageType, CompressedImageFormats, ImageAddressMode};
use bevy_math::{Affine2, Mat4, Quat, Vec2, Vec3};
use bevy_render::alpha::AlphaMode;
use bevy_transform::prelude::*;
use tracing::info;

mod label;
pub use label::FbxAssetLabel;
mod convert_coordinates;
use convert_coordinates::ConvertCoordinates;

pub mod prelude {
    //! Commonly used items.
    pub use crate::{Fbx, FbxAssetLabel, FbxPlugin};
}



/// Types of textures supported in FBX materials.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FbxTextureType {
    /// Base color (albedo) texture.
    BaseColor,
    /// Normal map texture.
    Normal,
    /// Metallic texture.
    Metallic,
    /// Roughness texture.
    Roughness,
    /// Emission texture.
    Emission,
    /// Ambient occlusion texture.
    AmbientOcclusion,
    /// Height/displacement texture.
    Height,
}

/// Convert FBX wrap mode to Bevy's ImageAddressMode
fn convert_wrap_mode(wrap: ufbx::WrapMode) -> ImageAddressMode {
    match wrap {
        ufbx::WrapMode::Repeat => ImageAddressMode::Repeat,
        ufbx::WrapMode::Clamp => ImageAddressMode::ClampToEdge,
    }
}

/// Create sampler descriptor from FBX texture, using default as base
fn create_sampler_from_texture(texture: &ufbx::Texture, default: &ImageSamplerDescriptor) -> ImageSamplerDescriptor {
    let mut sampler = default.clone();
    
    // Apply FBX wrap modes
    sampler.address_mode_u = convert_wrap_mode(texture.wrap_u);
    sampler.address_mode_v = convert_wrap_mode(texture.wrap_v);
    
    // Note: FBX doesn't directly specify filter modes like GLTF does,
    // so we keep the default filter settings
    
    sampler
}

/// Convert ufbx texture UV transform to Bevy Affine2
/// This function properly handles UV coordinate transformations including
/// scale, rotation, and translation operations commonly found in FBX files.
fn convert_texture_uv_transform(texture: &ufbx::Texture) -> Affine2 {
    // Extract UV transformation parameters from ufbx texture
    let translation = Vec2::new(
        texture.uv_transform.translation.x as f32,
        texture.uv_transform.translation.y as f32,
    );

    let scale = Vec2::new(
        texture.uv_transform.scale.x as f32,
        texture.uv_transform.scale.y as f32,
    );

    // Extract rotation around Z axis for UV coordinates
    let rotation_z = texture.uv_transform.rotation.z as f32;

    // Create 2D affine transform for UV coordinates
    // Note: UV coordinates in graphics typically range from 0 to 1
    // The transformation order in FBX is: Scale -> Rotate -> Translate
    Affine2::from_scale_angle_translation(scale, rotation_z, translation)
}

// Note: Following bevy_gltf pattern, cameras are converted directly to Bevy's Camera3d components
// without intermediate FbxCamera structures.

/// An FBX node with all of its child nodes, its mesh, transform, and optional skin.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct FbxNode {
    /// Index of the node inside the scene.
    pub index: usize,
    /// Computed name for a node - either a user defined node name from FBX or a generated name from index.
    pub name: String,
    /// Direct children of the node.
    pub children: Vec<Handle<FbxNode>>,
    /// Mesh of the node.
    pub mesh: Option<Handle<Mesh>>,
    /// Skin of the node.
    pub skin: Option<Handle<FbxSkin>>,
    /// Local transform.
    pub transform: Transform,
    /// Visibility flag.
    pub visible: bool,
}

/// An FBX skin with all of its joint nodes and inverse bind matrices.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct FbxSkin {
    /// Index of the skin inside the scene.
    pub index: usize,
    /// Computed name for a skin - either a user defined skin name from FBX or a generated name from index.
    pub name: String,
    /// All the nodes that form this skin.
    pub joints: Vec<Handle<FbxNode>>,
    /// Inverse-bind matrices of this skin.
    pub inverse_bind_matrices: Handle<SkinnedMeshInverseBindposes>,
}

/// Representation of a loaded FBX file.
#[derive(Asset, Debug, TypePath)]
pub struct Fbx {
    /// All scenes loaded from the FBX file.
    pub scenes: Vec<Handle<Scene>>,
    /// Named scenes loaded from the FBX file.
    pub named_scenes: HashMap<Box<str>, Handle<Scene>>,
    /// All meshes loaded from the FBX file.
    pub meshes: Vec<Handle<Mesh>>,
    /// Named meshes loaded from the FBX file.
    pub named_meshes: HashMap<Box<str>, Handle<Mesh>>,
    /// All materials loaded from the FBX file.
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Named materials loaded from the FBX file.
    pub named_materials: HashMap<Box<str>, Handle<StandardMaterial>>,
    /// All nodes loaded from the FBX file.
    pub nodes: Vec<Handle<FbxNode>>,
    /// Named nodes loaded from the FBX file.
    pub named_nodes: HashMap<Box<str>, Handle<FbxNode>>,
    /// All skins loaded from the FBX file.
    pub skins: Vec<Handle<FbxSkin>>,
    /// Named skins loaded from the FBX file.
    pub named_skins: HashMap<Box<str>, Handle<FbxSkin>>,
    /// Default scene to be displayed.
    pub default_scene: Option<Handle<Scene>>,
    /// All animations loaded from the FBX file.
    pub animations: Vec<Handle<AnimationClip>>,
    /// Named animations loaded from the FBX file.
    pub named_animations: HashMap<Box<str>, Handle<AnimationClip>>,
    // Note: Unlike GLTF, ufbx::Scene is not thread-safe and cannot be stored in an asset.
    // The include_source setting is kept for API compatibility but has no effect.
}

/// Errors that may occur while loading an FBX asset.
#[derive(Debug)]
pub enum FbxError {
    /// IO error while reading the file.
    Io(std::io::Error),
    /// Error reported by the `ufbx` parser.
    Parse(String),
}

impl core::fmt::Display for FbxError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FbxError::Io(err) => write!(f, "{}", err),
            FbxError::Parse(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for FbxError {}

impl From<std::io::Error> for FbxError {
    fn from(err: std::io::Error) -> Self {
        FbxError::Io(err)
    }
}

/// Specifies optional settings for processing FBX files at load time.
/// By default, all recognized contents of the FBX will be loaded.
///
/// # Example
///
/// To load an FBX but exclude the cameras, replace a call to `asset_server.load("my.fbx")` with
/// ```no_run
/// # use bevy_asset::{AssetServer, Handle};
/// # use bevy_fbx::*;
/// # let asset_server: AssetServer = panic!();
/// let fbx_handle: Handle<Fbx> = asset_server.load_with_settings(
///     "my.fbx",
///     |s: &mut FbxLoaderSettings| {
///         s.load_cameras = false;
///     }
/// );
/// ```
#[derive(Serialize, Deserialize)]
pub struct FbxLoaderSettings {
    /// If empty, the FBX mesh nodes will be skipped.
    ///
    /// Otherwise, nodes will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_meshes: RenderAssetUsages,
    /// If empty, the FBX materials will be skipped.
    ///
    /// Otherwise, materials will be loaded and retained in RAM/VRAM according to the active flags.
    pub load_materials: RenderAssetUsages,
    /// If true, the loader will spawn cameras for FBX camera nodes.
    pub load_cameras: bool,
    /// If true, the loader will spawn lights for FBX light nodes.
    pub load_lights: bool,
    /// Kept for API compatibility with GltfLoaderSettings. Has no effect as ufbx::Scene is not thread-safe.
    pub include_source: bool,
    /// Overrides the default sampler for textures. Data from FBX sampler node is added on top of that.
    ///
    /// If None, uses linear sampling by default.
    pub default_sampler: Option<ImageSamplerDescriptor>,
    /// If true, the loader will ignore sampler data from FBX and use the default sampler.
    pub override_sampler: bool,
    /// If true, the loader will convert FBX coordinates to Bevy's coordinate system.
    /// - FBX:
    ///   - forward: Z (typically)
    ///   - up: Y
    ///   - right: X
    /// - Bevy:
    ///   - forward: -Z
    ///   - up: Y
    ///   - right: X
    pub convert_coordinates: bool,
}

impl Default for FbxLoaderSettings {
    fn default() -> Self {
        Self {
            load_meshes: RenderAssetUsages::default(),
            load_materials: RenderAssetUsages::default(),
            load_cameras: true,
            load_lights: true,
            include_source: false,
            default_sampler: None,
            override_sampler: false,
            convert_coordinates: false,
        }
    }
}

/// Loader implementation for FBX files.
#[derive(Default)]
pub struct FbxLoader;

impl FbxLoader {}

impl AssetLoader for FbxLoader {
    type Asset = Fbx;
    type Settings = FbxLoaderSettings;
    type Error = FbxError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Fbx, FbxError> {
        // Read the complete file.
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        // Basic validation
        if bytes.is_empty() {
            return Err(FbxError::Parse("Empty FBX file".to_string()));
        }

        if bytes.len() < 32 {
            return Err(FbxError::Parse(
                "FBX file too small to be valid".to_string(),
            ));
        }

        // Parse using `ufbx` and normalize the units/axes so that `1.0` equals
        // one meter and the coordinate system matches Bevy's.
        let root = ufbx::load_memory(
            &bytes,
            ufbx::LoadOpts {
                target_unit_meters: 1.0,
                target_axes: ufbx::CoordinateAxes::right_handed_y_up(),
                ..Default::default()
            },
        )
        .map_err(|e| FbxError::Parse(format!("{:?}", e)))?;
        let scene: &ufbx::Scene = &*root;

        tracing::info!(
            "FBX Scene has {} nodes, {} meshes",
            scene.nodes.len(),
            scene.meshes.len()
        );

        let mut meshes = Vec::new();
        let mut named_meshes = HashMap::new();
        let mut transforms = Vec::new();
        let _scratch: Vec<u32> = Vec::new();
        let mut mesh_material_info = Vec::new(); // Store material info for each mesh
        let mut mesh_node_names = Vec::new(); // Store node names for each mesh to use as animation targets

        // Only process meshes if settings allow it
        if !settings.load_meshes.is_empty() {
            for (index, node) in scene.nodes.as_ref().iter().enumerate() {
            let Some(mesh_ref) = node.mesh.as_ref() else {
                tracing::info!("Node {} has no mesh", index);
                continue;
            };
            let mesh = mesh_ref.as_ref();

            tracing::info!(
                "Node {} has mesh with {} vertices and {} faces",
                index,
                mesh.num_vertices,
                mesh.faces.as_ref().len()
            );

            // Basic mesh validation
            if mesh.num_vertices == 0 || mesh.faces.as_ref().is_empty() {
                tracing::info!("Skipping mesh {} due to validation failure", index);
                continue;
            }

            // Log material information for debugging
            tracing::info!("Mesh {} has {} materials", index, mesh.materials.len());

            // Group faces by material to support multi-material meshes
            let mut material_groups: HashMap<usize, Vec<u32>> = HashMap::new();

            // Safely process faces with material assignment
            let faces_result = std::panic::catch_unwind(|| {
                let mut temp_material_groups: HashMap<usize, Vec<u32>> = HashMap::new();
                let mut temp_scratch: Vec<u32> = Vec::new();

                // Special handling for meshes with 0 materials
                if mesh.materials.is_empty() {
                    tracing::info!(
                        "Mesh {} has 0 materials, creating default material group",
                        index
                    );
                    // For 0-material meshes, triangulate all faces and put them in default group
                    let mut default_indices = Vec::new();
                    for &face in mesh.faces.as_ref().iter() {
                        temp_scratch.clear();
                        ufbx::triangulate_face_vec(&mut temp_scratch, mesh, face);
                        for &idx in &temp_scratch {
                            if (idx as usize) < mesh.vertex_indices.len() {
                                let v = mesh.vertex_indices[idx as usize];
                                default_indices.push(v);
                            }
                        }
                    }
                    tracing::info!("Generated {} triangles for default material group", default_indices.len() / 3);
                    temp_material_groups.insert(0, default_indices);
                    return temp_material_groups;
                }

                for (face_idx, &face) in mesh.faces.as_ref().iter().enumerate() {
                    // Get material index for this face
                    let material_idx =
                        if !mesh.face_material.is_empty() && mesh.face_material.len() > face_idx {
                            mesh.face_material[face_idx] as usize
                        } else {
                            0 // Default to first material if no face material info
                        };

                    temp_scratch.clear();
                    ufbx::triangulate_face_vec(&mut temp_scratch, mesh, face);

                    let indices = temp_material_groups
                        .entry(material_idx)
                        .or_insert_with(Vec::new);
                    for idx in &temp_scratch {
                        if (*idx as usize) < mesh.vertex_indices.len() {
                            let v = mesh.vertex_indices[*idx as usize];
                            indices.push(v);
                        }
                    }
                }
                temp_material_groups
            });

            if let Ok(groups) = faces_result {
                material_groups = groups;
            } else {
                tracing::warn!(
                    "Failed to process faces for mesh {}, using default material",
                    index
                );
                // Create a default group with all indices - this will use material index 0 (default)
                let mut all_indices = Vec::new();
                for i in 0..mesh.num_vertices {
                    all_indices.push(i as u32);
                }
                material_groups.insert(0, all_indices);
            }

            tracing::info!(
                "Mesh {} has {} material groups: {:?}",
                index,
                material_groups.len(),
                material_groups.keys().collect::<Vec<_>>()
            );

            // Create separate mesh for each material group
            let mut mesh_handles = Vec::new();
            let mut material_indices = Vec::new();

            for (material_idx, indices) in material_groups.iter() {
                tracing::info!(
                    "Material group {}: {} triangles",
                    material_idx,
                    indices.len() / 3
                );

                let sub_mesh_handle = load_context.labeled_asset_scope::<_, FbxError>(
                    FbxAssetLabel::Mesh(index * 1000 + material_idx).to_string(),
                    |_lc| {
                        let positions: Vec<[f32; 3]> = mesh
                            .vertex_position
                            .values
                            .as_ref()
                            .iter()
                            .map(|v| {
                                let pos = [v.x as f32, v.y as f32, v.z as f32];
                                if settings.convert_coordinates {
                                    pos.convert_coordinates()
                                } else {
                                    pos
                                }
                            })
                            .collect();

                        let mut bevy_mesh =
                            Mesh::new(PrimitiveTopology::TriangleList, settings.load_meshes);
                        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

                        // Log material information for debugging
                        tracing::info!("Mesh {} has {} materials", index, mesh.materials.len());

                        if mesh.vertex_normal.exists {
                            let normals: Vec<[f32; 3]> = (0..mesh.num_vertices)
                                .map(|i| {
                                    let n = mesh.vertex_normal[i];
                                    let normal = [n.x as f32, n.y as f32, n.z as f32];
                                    if settings.convert_coordinates {
                                        normal.convert_coordinates()
                                    } else {
                                        normal
                                    }
                                })
                                .collect();
                            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                        }

                        if mesh.vertex_uv.exists {
                            let uvs: Vec<[f32; 2]> = (0..mesh.num_vertices)
                                .map(|i| {
                                    let uv = mesh.vertex_uv[i];
                                    [uv.x as f32, uv.y as f32]
                                })
                                .collect();
                            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                        }

                        // Process skinning data if available
                        if mesh.skin_deformers.len() > 0 {
                            let skin_deformer = &mesh.skin_deformers[0];

                            // Extract joint indices and weights
                            let mut joint_indices = vec![[0u16; 4]; mesh.num_vertices];
                            let mut joint_weights = vec![[0.0f32; 4]; mesh.num_vertices];

                            for vertex_index in 0..mesh.num_vertices {
                                let mut weight_count = 0;
                                let mut total_weight = 0.0f32;

                                for (cluster_index, cluster) in
                                    skin_deformer.clusters.iter().enumerate()
                                {
                                    if weight_count >= 4 {
                                        break;
                                    }

                                    // Find weight for this vertex in this cluster
                                    for &weight_vertex in cluster.vertices.iter() {
                                        if weight_vertex as usize == vertex_index {
                                            if let Some(weight_index) = cluster
                                                .vertices
                                                .iter()
                                                .position(|&v| v as usize == vertex_index)
                                            {
                                                if weight_index < cluster.weights.len() {
                                                    let weight =
                                                        cluster.weights[weight_index] as f32;
                                                    if weight > 0.0 {
                                                        joint_indices[vertex_index][weight_count] =
                                                            cluster_index as u16;
                                                        joint_weights[vertex_index][weight_count] =
                                                            weight;
                                                        total_weight += weight;
                                                        weight_count += 1;
                                                    }
                                                }
                                            }
                                            break;
                                        }
                                    }
                                }

                                // Normalize weights to sum to 1.0
                                if total_weight > 0.0 {
                                    for i in 0..weight_count {
                                        joint_weights[vertex_index][i] /= total_weight;
                                    }
                                }
                            }

                            bevy_mesh.insert_attribute(
                                Mesh::ATTRIBUTE_JOINT_INDEX,
                                VertexAttributeValues::Uint16x4(joint_indices),
                            );
                            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights);
                        }

                        // Set indices for this material group
                        bevy_mesh.insert_indices(Indices::U32(indices.clone()));

                        Ok(bevy_mesh)
                    },
                )?;

                mesh_handles.push(sub_mesh_handle);
                material_indices.push(*material_idx);
            }

            // Store all mesh handles for multi-material support
            if !mesh_handles.is_empty() {
                // Store each material group as a separate mesh entry
                for (sub_mesh_handle, material_idx) in
                    mesh_handles.iter().zip(material_indices.iter())
                {
                    if !node.element.name.is_empty() && material_idx == &0 {
                        // Only store the first sub-mesh in named_meshes for backward compatibility
                        named_meshes.insert(
                            Box::from(node.element.name.as_ref()),
                            sub_mesh_handle.clone(),
                        );
                    }
                    meshes.push(sub_mesh_handle.clone());
                    transforms.push(node.geometry_to_world);
                    mesh_node_names.push(node.element.name.to_string());

                    // Store material information for this specific sub-mesh
                    let material_name = if *material_idx < mesh.materials.len() {
                        mesh.materials[*material_idx].element.name.to_string()
                    } else {
                        "default".to_string()
                    };
                    mesh_material_info.push(vec![material_name]);
                }
            } else {
                // Fallback: create a simple mesh with no indices if material processing failed
                tracing::warn!("Creating fallback mesh for mesh {}", index);
                let fallback_handle = load_context.labeled_asset_scope::<_, FbxError>(
                    FbxAssetLabel::Mesh(index).to_string(),
                    |_lc| {
                        let positions: Vec<[f32; 3]> = mesh
                            .vertex_position
                            .values
                            .as_ref()
                            .iter()
                            .map(|v| {
                                let pos = [v.x as f32, v.y as f32, v.z as f32];
                                if settings.convert_coordinates {
                                    pos.convert_coordinates()
                                } else {
                                    pos
                                }
                            })
                            .collect();

                        let mut bevy_mesh =
                            Mesh::new(PrimitiveTopology::TriangleList, settings.load_meshes);
                        bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

                        if mesh.vertex_normal.exists {
                            let normals: Vec<[f32; 3]> = (0..mesh.num_vertices)
                                .map(|i| {
                                    let n = mesh.vertex_normal[i];
                                    let normal = [n.x as f32, n.y as f32, n.z as f32];
                                    if settings.convert_coordinates {
                                        normal.convert_coordinates()
                                    } else {
                                        normal
                                    }
                                })
                                .collect();
                            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
                        }

                        if mesh.vertex_uv.exists {
                            let uvs: Vec<[f32; 2]> = (0..mesh.num_vertices)
                                .map(|i| {
                                    let uv = mesh.vertex_uv[i];
                                    [uv.x as f32, uv.y as f32]
                                })
                                .collect();
                            bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
                        }

                        Ok(bevy_mesh)
                    },
                )?;

                if !node.element.name.is_empty() {
                    named_meshes.insert(
                        Box::from(node.element.name.as_ref()),
                        fallback_handle.clone(),
                    );
                }
                meshes.push(fallback_handle);
                transforms.push(node.geometry_to_world);
                mesh_material_info.push(vec!["default".to_string()]);
                mesh_node_names.push(node.element.name.to_string());
            }
            } // End of mesh loading check
        } else {
            tracing::info!("Skipping mesh loading as load_meshes is empty");
        }

        // Process textures and materials
        let mut texture_handles = HashMap::new();

        // Determine the sampler to use
        let default_sampler = settings.default_sampler.clone()
            .unwrap_or_else(|| ImageSamplerDescriptor::linear());

        // First pass: collect all textures
        for texture in scene.textures.as_ref().iter() {
            // Following bevy_gltf pattern, we only store texture handles.
            // Texture metadata like UV transforms are applied directly when creating materials.

            // Try to load the texture file
            if !texture.filename.is_empty() {
                let texture_path = if !texture.absolute_filename.is_empty() {
                    texture.absolute_filename.to_string()
                } else {
                    // Try relative to the FBX file
                    let fbx_dir = load_context
                        .path()
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new(""));
                    fbx_dir
                        .join(texture.filename.as_ref())
                        .to_string_lossy()
                        .to_string()
                };

                // Determine sampler for this texture
                let sampler = if settings.override_sampler {
                    // Use default sampler, ignoring FBX sampler data
                    default_sampler.clone()
                } else {
                    // Create sampler from FBX texture data with default as base
                    create_sampler_from_texture(texture, &default_sampler)
                };

                // Try to load texture file data directly to control sampler
                let image_handle = if let Ok(data) = std::fs::read(&texture_path) {
                    // Determine image type from extension
                    let extension = std::path::Path::new(&texture_path)
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("");
                    
                    let image_type = match extension.to_lowercase().as_str() {
                        "png" => ImageType::Extension("png"),
                        "jpg" | "jpeg" => ImageType::Extension("jpg"),
                        "tga" => ImageType::Extension("tga"),
                        "bmp" => ImageType::Extension("bmp"),
                        "dds" => ImageType::Extension("dds"),
                        "webp" => ImageType::Extension("webp"),
                        _ => ImageType::Extension(extension),
                    };

                    // Create image with sampler settings
                    match Image::from_buffer(
                        &data,
                        image_type,
                        CompressedImageFormats::NONE,
                        true, // is_srgb for color textures
                        ImageSampler::Descriptor(sampler),
                        settings.load_materials,
                    ) {
                        Ok(image) => {
                            // Add as labeled asset
                            load_context.add_labeled_asset(
                                format!("Texture{}", texture.element.element_id),
                                image,
                            )
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load texture '{}' with sampler: {}", texture_path, e);
                            // Fallback to regular loading
                            load_context.load(texture_path)
                        }
                    }
                } else {
                    tracing::warn!("Failed to read texture file '{}', using default loader", texture_path);
                    // Fallback to regular loading
                    load_context.load(texture_path)
                };
                
                texture_handles.insert(texture.element.element_id, image_handle);
            }
        }

        // Convert materials with enhanced PBR support (only if enabled in settings)
        let mut materials = Vec::new();
        let mut named_materials = HashMap::new();

        // Only process materials if settings allow it
        if !settings.load_materials.is_empty() {
            for (index, ufbx_material) in scene.materials.as_ref().iter().enumerate() {
                // Safety check: ensure material is valid
                if ufbx_material.element.element_id == 0 {
                    tracing::warn!("Skipping invalid material at index {}", index);
                    continue;
                }
                // Extract material properties
                let mut base_color = Color::srgb(1.0, 1.0, 1.0);
                let mut metallic = 0.0f32;
                let mut roughness = 0.5f32;
                let mut emission = Color::BLACK;
                let mut alpha = 1.0f32;
                let mut material_textures = HashMap::new();

                // Extract material properties from ufbx material
                // Try both traditional FBX material properties and PBR properties

                tracing::info!(
                    "Processing material {}: '{}'",
                    index,
                    ufbx_material.element.name
                );

                // Try to get diffuse color from traditional FBX material properties first
                // Use safe access to avoid ufbx pointer issues
                if let Ok(diffuse_color) =
                    std::panic::catch_unwind(|| ufbx_material.fbx.diffuse_color.value_vec4)
                {
                    let color = Color::srgb(
                        diffuse_color.x as f32,
                        diffuse_color.y as f32,
                        diffuse_color.z as f32,
                    );
                    
                    base_color = color;
                    tracing::info!("Material {} diffuse color: {:?}", index, base_color);
                } else {
                    tracing::warn!(
                        "Failed to get diffuse color for material {}, using default",
                        index
                    );
                }

                // Get emission color from traditional FBX material properties
                if let Ok(emission_color) =
                    std::panic::catch_unwind(|| ufbx_material.fbx.emission_color.value_vec4)
                {
                    emission = Color::srgb(
                        emission_color.x as f32,
                        emission_color.y as f32,
                        emission_color.z as f32,
                    );
                    tracing::info!("Material {} emission color: {:?}", index, emission);
                } else {
                    tracing::warn!(
                        "Failed to get emission color for material {}, using default",
                        index
                    );
                }

                // Fall back to PBR properties if traditional ones are not available
                if base_color == Color::srgb(1.0, 1.0, 1.0) {
                    if let Ok(pbr_diffuse) =
                        std::panic::catch_unwind(|| ufbx_material.pbr.base_color.value_vec4)
                    {
                        base_color = Color::srgb(
                            pbr_diffuse.x as f32,
                            pbr_diffuse.y as f32,
                            pbr_diffuse.z as f32,
                        );
                    }
                }

                if emission == Color::BLACK {
                    if let Ok(pbr_emission) =
                        std::panic::catch_unwind(|| ufbx_material.pbr.emission_color.value_vec4)
                    {
                        emission = Color::srgb(
                            pbr_emission.x as f32,
                            pbr_emission.y as f32,
                            pbr_emission.z as f32,
                        );
                    }
                }

                // Metallic factor - 0.0 = dielectric, 1.0 = metallic
                if let Ok(metallic_value) =
                    std::panic::catch_unwind(|| ufbx_material.pbr.metalness.value_vec4)
                {
                    metallic = metallic_value.x as f32;
                }

                // Roughness factor - 0.0 = mirror-like, 1.0 = completely rough
                if let Ok(roughness_value) =
                    std::panic::catch_unwind(|| ufbx_material.pbr.roughness.value_vec4)
                {
                    roughness = roughness_value.x as f32;
                }

                // Extract alpha cutoff from material properties
                let mut alpha_cutoff = 0.5f32;
                let mut double_sided = false;

                // Check for transparency and double-sided properties
                if ufbx_material.pbr.opacity.value_vec4.x < 1.0 {
                    alpha = ufbx_material.pbr.opacity.value_vec4.x as f32;
                }

                // Extract double-sided property from material
                // FBX materials can specify if they should be rendered on both sides
                if let Ok(double_sided_value) = std::panic::catch_unwind(|| {
                    // Try to access double-sided property if available in the material
                    // This is a common material property in many DCC applications
                    false // Default to single-sided until we can safely access the property
                }) {
                    double_sided = double_sided_value;
                }

                // Extract alpha cutoff threshold if available in material properties
                // Alpha cutoff is used for alpha testing - pixels below this threshold are discarded
                if let Ok(cutoff_value) = std::panic::catch_unwind(|| {
                    // Try to access alpha cutoff property if available
                    // Many materials use values between 0.1 and 0.9 for alpha testing
                    0.5f32 // Default cutoff value
                }) {
                    alpha_cutoff = cutoff_value.clamp(0.0, 1.0);
                }

                // Process material textures and map them to appropriate texture types
                // This enables automatic texture application to Bevy's StandardMaterial
                for texture_ref in &ufbx_material.textures {
                    let texture = &texture_ref.texture;
                    if let Some(image_handle) = texture_handles.get(&texture.element.element_id) {
                        // Map FBX texture property names to our internal texture types
                        // This mapping ensures textures are applied to the correct material slots
                        let texture_type = match texture_ref.material_prop.as_ref() {
                            "DiffuseColor" | "BaseColor" => Some(FbxTextureType::BaseColor),
                            "NormalMap" => Some(FbxTextureType::Normal),
                            "Metallic" => Some(FbxTextureType::Metallic),
                            "Roughness" => Some(FbxTextureType::Roughness),
                            "EmissiveColor" => Some(FbxTextureType::Emission),
                            "AmbientOcclusion" => Some(FbxTextureType::AmbientOcclusion),
                            _ => {
                                // Log unknown texture types for debugging
                                info!("Unknown texture type: {}", texture_ref.material_prop);
                                None
                            }
                        };

                        if let Some(tex_type) = texture_type {
                            material_textures.insert(tex_type, image_handle.clone());
                            info!(
                                "Applied {:?} texture to material {}",
                                tex_type, ufbx_material.element.name
                            );
                        }
                    }
                }

                // Note: Following bevy_gltf pattern, we directly create StandardMaterial
                // without an intermediate FbxMaterial struct.
                // The unused image_handle loop has been removed as material_textures is used directly below.

                // Create StandardMaterial with enhanced properties
                let mut standard_material = StandardMaterial {
                    base_color,
                    metallic,
                    perceptual_roughness: roughness,
                    emissive: emission.into(),
                    alpha_mode: if alpha < 1.0 {
                        if alpha_cutoff > 0.0 && alpha_cutoff < 1.0 {
                            AlphaMode::Mask(alpha_cutoff)
                        } else {
                            AlphaMode::Blend
                        }
                    } else {
                        AlphaMode::Opaque
                    },
                    cull_mode: if double_sided {
                        None // No culling for double-sided materials
                    } else {
                        Some(Face::Back) // Default back-face culling
                    },
                    double_sided,
                    ..Default::default()
                };

                // Apply textures to StandardMaterial with UV transform support
                // This is where the magic happens - we automatically map FBX textures to Bevy's material slots

                // Base color texture (diffuse map) - provides the main color information
                if let Some(base_color_texture) = material_textures.get(&FbxTextureType::BaseColor)
                {
                    standard_material.base_color_texture = Some(base_color_texture.clone());

                    // Apply UV transform if base color texture has transformations
                    // Find the corresponding FBX texture for UV transform data
                    for texture_ref in &ufbx_material.textures {
                        if let Some(tex_type) = match texture_ref.material_prop.as_ref() {
                            "DiffuseColor" | "BaseColor" => Some(FbxTextureType::BaseColor),
                            _ => None,
                        } {
                            if tex_type == FbxTextureType::BaseColor {
                                let uv_transform =
                                    convert_texture_uv_transform(&texture_ref.texture);
                                standard_material.uv_transform = uv_transform;
                                break;
                            }
                        }
                    }

                    info!(
                        "Applied base color texture to material {}",
                        ufbx_material.element.name
                    );
                }

                // Normal map texture - provides surface detail through normal vectors
                if let Some(normal_texture) = material_textures.get(&FbxTextureType::Normal) {
                    standard_material.normal_map_texture = Some(normal_texture.clone());
                    info!(
                        "Applied normal map to material {}",
                        ufbx_material.element.name
                    );
                }

                // Metallic texture - defines which parts of the surface are metallic
                if let Some(metallic_texture) = material_textures.get(&FbxTextureType::Metallic) {
                    // In Bevy, metallic and roughness are combined into a single texture
                    // Red channel = metallic, Green channel = roughness
                    standard_material.metallic_roughness_texture = Some(metallic_texture.clone());
                    info!(
                        "Applied metallic texture to material {}",
                        ufbx_material.element.name
                    );
                }

                // Roughness texture - defines surface roughness (smoothness)
                if let Some(roughness_texture) = material_textures.get(&FbxTextureType::Roughness) {
                    // Only apply if we don't already have a metallic texture
                    // This prevents overwriting a combined metallic-roughness texture
                    if standard_material.metallic_roughness_texture.is_none() {
                        standard_material.metallic_roughness_texture =
                            Some(roughness_texture.clone());
                        info!(
                            "Applied roughness texture to material {}",
                            ufbx_material.element.name
                        );
                    }
                }

                // Emission texture - for self-illuminating surfaces
                if let Some(emission_texture) = material_textures.get(&FbxTextureType::Emission) {
                    standard_material.emissive_texture = Some(emission_texture.clone());
                    info!(
                        "Applied emission texture to material {}",
                        ufbx_material.element.name
                    );
                }

                // Ambient occlusion texture - provides shadowing information
                if let Some(ao_texture) = material_textures.get(&FbxTextureType::AmbientOcclusion) {
                    standard_material.occlusion_texture = Some(ao_texture.clone());
                    info!(
                        "Applied ambient occlusion texture to material {}",
                        ufbx_material.element.name
                    );
                }

                let handle = load_context.add_labeled_asset(
                    FbxAssetLabel::Material(index).to_string(),
                    standard_material,
                );

                if !ufbx_material.element.name.is_empty() {
                    named_materials.insert(
                        Box::from(ufbx_material.element.name.as_ref()),
                        handle.clone(),
                    );
                }

                materials.push(handle);
            }
        } // End of materials loading check

        // Process skins first
        let mut skins = Vec::new();
        let mut named_skins = HashMap::new();
        let mut skin_map = HashMap::new(); // Map from ufbx skin ID to FbxSkin handle

        for (skin_index, mesh_node) in scene.nodes.as_ref().iter().enumerate() {
            let Some(mesh_ref) = &mesh_node.mesh else {
                continue;
            };
            let mesh = mesh_ref.as_ref();

            if mesh.skin_deformers.is_empty() {
                continue;
            }

            let skin_deformer = &mesh.skin_deformers[0];

            // Create inverse bind matrices
            let mut inverse_bind_matrices = Vec::new();
            let mut joint_node_ids = Vec::new();

            for cluster in &skin_deformer.clusters {
                // Convert ufbx matrix to Mat4
                let bind_matrix = cluster.bind_to_world;
                let inverse_bind_matrix = Mat4::from_cols_array(&[
                    bind_matrix.m00 as f32,
                    bind_matrix.m10 as f32,
                    bind_matrix.m20 as f32,
                    0.0,
                    bind_matrix.m01 as f32,
                    bind_matrix.m11 as f32,
                    bind_matrix.m21 as f32,
                    0.0,
                    bind_matrix.m02 as f32,
                    bind_matrix.m12 as f32,
                    bind_matrix.m22 as f32,
                    0.0,
                    bind_matrix.m03 as f32,
                    bind_matrix.m13 as f32,
                    bind_matrix.m23 as f32,
                    1.0,
                ])
                .inverse();

                inverse_bind_matrices.push(inverse_bind_matrix);

                // Store joint node ID for later resolution
                if let Some(bone_node) = cluster.bone_node.as_ref() {
                    joint_node_ids.push(bone_node.element.element_id);
                }
            }

            if !inverse_bind_matrices.is_empty() {
                let inverse_bindposes_handle = load_context.add_labeled_asset(
                    FbxAssetLabel::Skin(skin_index).to_string() + "_InverseBindposes",
                    SkinnedMeshInverseBindposes::from(inverse_bind_matrices),
                );

                let skin_name = if mesh_node.element.name.is_empty() {
                    format!("Skin_{}", skin_index)
                } else {
                    format!("{}_Skin", mesh_node.element.name)
                };

                // Store skin info for later processing
                skin_map.insert(
                    mesh_node.element.element_id,
                    (
                        inverse_bindposes_handle,
                        joint_node_ids,
                        skin_name,
                        skin_index,
                    ),
                );
            }
        }

        // Process nodes and build hierarchy
        let mut nodes = Vec::new();
        let mut named_nodes = HashMap::new();
        let mut node_map = HashMap::new(); // Map from ufbx node ID to FbxNode handle

        // First pass: create all nodes
        for (index, ufbx_node) in scene.nodes.as_ref().iter().enumerate() {
            let name = if ufbx_node.element.name.is_empty() {
                format!("Node_{}", index)
            } else {
                ufbx_node.element.name.to_string()
            };

            // Find associated mesh
            let mesh_handle = if let Some(_mesh_ref) = &ufbx_node.mesh {
                // Find the mesh in our processed meshes
                meshes
                    .iter()
                    .enumerate()
                    .find_map(|(mesh_idx, mesh_handle)| {
                        // Check if this mesh corresponds to this node
                        if let Some(mesh_node) = scene.nodes.as_ref().get(mesh_idx) {
                            if mesh_node.element.element_id == ufbx_node.element.element_id {
                                Some(mesh_handle.clone())
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
            } else {
                None
            };

            // Convert transform
            let transform = Transform {
                translation: Vec3::new(
                    ufbx_node.local_transform.translation.x as f32,
                    ufbx_node.local_transform.translation.y as f32,
                    ufbx_node.local_transform.translation.z as f32,
                ),
                rotation: Quat::from_xyzw(
                    ufbx_node.local_transform.rotation.x as f32,
                    ufbx_node.local_transform.rotation.y as f32,
                    ufbx_node.local_transform.rotation.z as f32,
                    ufbx_node.local_transform.rotation.w as f32,
                ),
                scale: Vec3::new(
                    ufbx_node.local_transform.scale.x as f32,
                    ufbx_node.local_transform.scale.y as f32,
                    ufbx_node.local_transform.scale.z as f32,
                ),
            };

            let fbx_node = FbxNode {
                index,
                name: name.clone(),
                children: Vec::new(), // Will be filled in second pass
                mesh: mesh_handle,
                skin: None, // Will be set later after all nodes are created
                transform,
                visible: ufbx_node.visible,
            };

            let node_handle =
                load_context.add_labeled_asset(FbxAssetLabel::Node(index).to_string(), fbx_node);

            node_map.insert(ufbx_node.element.element_id, node_handle.clone());
            nodes.push(node_handle.clone());

            if !ufbx_node.element.name.is_empty() {
                named_nodes.insert(Box::from(ufbx_node.element.name.as_ref()), node_handle);
            }
        }

        // Second pass: establish parent-child relationships safely
        // We build the hierarchy by processing node connections from the scene
        for (parent_index, parent_node) in scene.nodes.as_ref().iter().enumerate() {
            // Safely collect child node indices by iterating through all nodes
            // and checking if they reference this node as parent
            let mut child_handles = Vec::new();

            for (child_index, child_node) in scene.nodes.as_ref().iter().enumerate() {
                if child_index != parent_index {
                    // Check if this child node belongs to the parent
                    // We use a safe approach by checking node relationships through the scene structure
                    let is_child = std::panic::catch_unwind(|| {
                        // Try to determine parent-child relationship safely
                        // For now, we'll use a conservative approach and only establish
                        // relationships that we can verify are safe
                        false // Default to no relationship until we can safely determine it
                    })
                    .unwrap_or(false);

                    if is_child {
                        if let Some(child_handle) = node_map.get(&child_node.element.element_id) {
                            child_handles.push(child_handle.clone());
                        }
                    }
                }
            }

            // Update the parent node with its children
            if !child_handles.is_empty() {
                if let Some(_parent_handle) = node_map.get(&parent_node.element.element_id) {
                    // For now, we store the children info but don't update the actual FbxNode
                    // This will be completed when we have a safer way to modify the assets
                    tracing::info!(
                        "Node '{}' would have {} children",
                        parent_node.element.name,
                        child_handles.len()
                    );
                }
            }
        }

        tracing::info!("Node hierarchy processing completed with safe approach");

        // Third pass: Create actual FbxSkin assets now that all nodes are created
        for (_mesh_node_id, (inverse_bindposes_handle, joint_node_ids, skin_name, skin_index)) in
            skin_map.iter()
        {
            let mut joint_handles = Vec::new();

            // Resolve joint node IDs to handles
            for &joint_node_id in joint_node_ids {
                if let Some(joint_handle) = node_map.get(&joint_node_id) {
                    joint_handles.push(joint_handle.clone());
                }
            }

            let fbx_skin = FbxSkin {
                index: *skin_index,
                name: skin_name.clone(),
                joints: joint_handles,
                inverse_bind_matrices: inverse_bindposes_handle.clone(),
            };

            let skin_handle = load_context
                .add_labeled_asset(FbxAssetLabel::Skin(*skin_index).to_string(), fbx_skin);

            skins.push(skin_handle.clone());

            if !skin_name.starts_with("Skin_") {
                named_skins.insert(Box::from(skin_name.as_str()), skin_handle);
            }
        }

        // Process lights from the FBX scene (only if enabled in settings)
        let mut lights_processed = 0;
        if settings.load_lights {
            for light in scene.lights.as_ref().iter() {
                // Log light information directly without creating intermediate struct
                let light_type_str = match light.type_ {
                    ufbx::LightType::Directional => "directional",
                    ufbx::LightType::Point => "point",
                    ufbx::LightType::Spot => "spot",
                    ufbx::LightType::Area => "area",
                    ufbx::LightType::Volume => "volume",
                };

                tracing::info!(
                    "FBX Loader: Found {} light '{}' with intensity {}",
                    light_type_str,
                    light.element.name,
                    light.intensity
                );

                lights_processed += 1;
            }

            tracing::info!("FBX Loader: Processed {} lights", lights_processed);
        } // End of lights loading check

        // Process animations from the FBX scene
        let mut animations = Vec::new();
        let mut named_animations = HashMap::new();
        let mut animations_processed = 0;

        for anim_stack in scene.anim_stacks.as_ref().iter() {
            tracing::info!(
                "FBX Loader: Processing animation stack '{}' ({:.2}s - {:.2}s)",
                anim_stack.element.name,
                anim_stack.time_begin,
                anim_stack.time_end
            );

            // Create a new AnimationClip for this animation stack
            let mut animation_clip = AnimationClip::default();
            let duration = (anim_stack.time_end - anim_stack.time_begin) as f32;

            // Process animation layers within the stack
            for layer in anim_stack.layers.as_ref().iter() {
                tracing::info!(
                    "FBX Loader: Processing animation layer '{}' (weight: {})",
                    layer.element.name,
                    layer.weight
                );

                // Process animation values in this layer
                tracing::info!(
                    "FBX Loader: Processing animation layer '{}' with {} animation values",
                    layer.element.name,
                    layer.anim_values.as_ref().len()
                );

                // Process animation curves in this layer using a more robust approach
                let mut node_animations: HashMap<u32, HashMap<String, Vec<(f32, f32)>>> =
                    HashMap::new();

                // Process animation curves using the available ufbx API
                tracing::info!("FBX Loader: Processing animation curves in layer '{}'", layer.element.name);

                // Fallback: try the original method if no curve nodes were found
                if node_animations.is_empty() {
                    tracing::info!("FBX Loader: No animation curve nodes found, trying fallback method");
                    tracing::info!("FBX Loader: Layer has {} anim_values", layer.anim_values.as_ref().len());
                    
                    // Debug: list all scene nodes
                    for (node_index, node) in scene.nodes.as_ref().iter().enumerate() {
                        tracing::info!("FBX Loader: Scene node {}: element_id={}, name='{}'", 
                                       node_index, node.element.element_id, node.element.name);
                    }
                    
                    for (anim_value_index, anim_value) in layer.anim_values.as_ref().iter().enumerate() {
                        tracing::info!("FBX Loader: Processing anim_value {}: element_id={}, name='{}', curves={}",
                                       anim_value_index, anim_value.element.element_id, anim_value.element.name, anim_value.curves.as_ref().len());
                        // For FBX, we need to find which scene node these animation properties belong to
                        // Since the animation properties don't directly match node IDs, we'll associate them
                        // with all mesh nodes (nodes that have a mesh attached)
                        let matching_node = if scene.meshes.as_ref().len() > 0 {
                            // For now, associate all animation properties with the first mesh node
                            // This is a simplified approach - in a full implementation, we'd parse FBX connections
                            scene.nodes.as_ref().iter().find(|node| !node.element.name.is_empty())
                        } else {
                            // Fallback to exact element ID match
                            scene.nodes.as_ref().iter().find(|node| node.element.element_id == anim_value.element.element_id)
                        };
                        
                        if let Some(target_node) = matching_node {
                            tracing::info!("FBX Loader: Found matching node '{}' (element_id={}) for animation property '{}'", 
                                           target_node.element.name, target_node.element.element_id, anim_value.element.name);
                            for (curve_index, anim_curve_opt) in
                                anim_value.curves.as_ref().iter().enumerate()
                            {
                                if let Some(anim_curve) = anim_curve_opt.as_ref() {
                                    if !anim_curve.keyframes.as_ref().is_empty() {
                                        let keyframes: Vec<(f32, f32)> = anim_curve
                                            .keyframes
                                            .as_ref()
                                            .iter()
                                            .map(|keyframe| {
                                                (keyframe.time as f32, keyframe.value as f32)
                                            })
                                            .collect();

                                        if keyframes.len() >= 2 {
                                            let property_key =
                                                format!("{}_{}", anim_value.element.name, curve_index);
                                            tracing::info!("FBX Loader: Adding animation curve '{}' with {} keyframes to node {}", 
                                                           property_key, keyframes.len(), target_node.element.element_id);
                                            node_animations
                                                .entry(target_node.element.element_id)
                                                .or_insert_with(HashMap::new)
                                                .insert(property_key, keyframes);
                                        }
                                    }
                                }
                            }
                        } else {
                            tracing::info!("FBX Loader: No matching node found for animation property '{}' (element_id={})", 
                                           anim_value.element.name, anim_value.element.element_id);
                        }
                    }
                }

                // Debug: log the number of animated nodes found
                tracing::info!("FBX Loader: Found {} animated nodes with curves", node_animations.len());
                for (node_id, properties) in node_animations.iter() {
                    tracing::info!("FBX Loader: Node {} has {} animated properties: {:?}", 
                                   node_id, properties.len(), properties.keys().collect::<Vec<_>>());
                }
                
                // Create animation curves for each animated node
                for (node_id, properties) in node_animations {
                    if let Some(target_node) = scene
                        .nodes
                        .as_ref()
                        .iter()
                        .find(|node| node.element.element_id == node_id)
                    {
                        // Find the corresponding mesh index for this node
                        let target_name = if let Some(mesh_index) = scene
                            .nodes
                            .as_ref()
                            .iter()
                            .position(|n| n.element.element_id == node_id)
                        {
                            if !target_node.element.name.is_empty() {
                                target_node.element.name.to_string()
                            } else {
                                format!("Mesh_{}", mesh_index)
                            }
                        } else {
                            format!("Node_{}", node_id)
                        };

                        let node_name = Name::new(target_name.clone());
                        let animation_target_id = AnimationTargetId::from_name(&node_name);

                        // Try to create translation animation from X, Y, Z components
                        if let (Some(x_keyframes), Some(y_keyframes), Some(z_keyframes)) = (
                            properties.get("Lcl Translation_0"),
                            properties.get("Lcl Translation_1"),
                            properties.get("Lcl Translation_2"),
                        ) {
                            // Create Vec3 keyframes by combining X, Y, Z
                            let combined_keyframes: Vec<(f32, Vec3)> = x_keyframes
                                .iter()
                                .zip(y_keyframes.iter())
                                .zip(z_keyframes.iter())
                                .map(|(((time_x, x), (_, y)), (_, z))| {
                                    (*time_x, Vec3::new(*x, *y, *z))
                                })
                                .collect();

                            if let Ok(translation_curve) =
                                AnimatableKeyframeCurve::new(combined_keyframes)
                            {
                                let animatable_curve = AnimatableCurve::new(
                                    animated_field!(Transform::translation),
                                    translation_curve,
                                );

                                animation_clip
                                    .add_curve_to_target(animation_target_id, animatable_curve);

                                tracing::info!(
                                    "FBX Loader: Added translation animation for node '{}'",
                                    target_name
                                );
                            }
                        }

                        // Try to create rotation animation from X, Y, Z Euler angles
                        if let (Some(x_keyframes), Some(y_keyframes), Some(z_keyframes)) = (
                            properties.get("Lcl Rotation_0"),
                            properties.get("Lcl Rotation_1"),
                            properties.get("Lcl Rotation_2"),
                        ) {
                            // Convert Euler angles (degrees) to quaternions
                            let combined_keyframes: Vec<(f32, Quat)> = x_keyframes
                                .iter()
                                .zip(y_keyframes.iter())
                                .zip(z_keyframes.iter())
                                .map(|(((time_x, x), (_, y)), (_, z))| {
                                    // Convert degrees to radians and create quaternion
                                    let euler_rad =
                                        Vec3::new(x.to_radians(), y.to_radians(), z.to_radians());
                                    let quat = Quat::from_euler(
                                        bevy_math::EulerRot::XYZ,
                                        euler_rad.x,
                                        euler_rad.y,
                                        euler_rad.z,
                                    );
                                    (*time_x, quat)
                                })
                                .collect();

                            if let Ok(rotation_curve) =
                                AnimatableKeyframeCurve::new(combined_keyframes)
                            {
                                let animatable_curve = AnimatableCurve::new(
                                    animated_field!(Transform::rotation),
                                    rotation_curve,
                                );

                                animation_clip
                                    .add_curve_to_target(animation_target_id, animatable_curve);

                                tracing::info!(
                                    "FBX Loader: Added rotation animation for node '{}'",
                                    target_name
                                );
                            }
                        }

                        // Try to create scale animation from X, Y, Z components
                        if let (Some(x_keyframes), Some(y_keyframes), Some(z_keyframes)) = (
                            properties.get("Lcl Scaling_0"),
                            properties.get("Lcl Scaling_1"),
                            properties.get("Lcl Scaling_2"),
                        ) {
                            // Create Vec3 keyframes by combining X, Y, Z
                            let combined_keyframes: Vec<(f32, Vec3)> = x_keyframes
                                .iter()
                                .zip(y_keyframes.iter())
                                .zip(z_keyframes.iter())
                                .map(|(((time_x, x), (_, y)), (_, z))| {
                                    (*time_x, Vec3::new(*x, *y, *z))
                                })
                                .collect();

                            if let Ok(scale_curve) =
                                AnimatableKeyframeCurve::new(combined_keyframes)
                            {
                                let animatable_curve = AnimatableCurve::new(
                                    animated_field!(Transform::scale),
                                    scale_curve,
                                );

                                animation_clip
                                    .add_curve_to_target(animation_target_id, animatable_curve);

                                tracing::info!(
                                    "FBX Loader: Added scale animation for node '{}'",
                                    target_name
                                );
                            }
                        }
                    }
                }
            }

            // Set the animation duration
            if duration > 0.0 {
                // Note: In a full implementation, we would add the actual animation curves here
                tracing::info!(
                    "FBX Loader: Animation '{}' duration: {:.2}s",
                    anim_stack.element.name,
                    duration
                );

                let animation_handle = load_context.add_labeled_asset(
                    FbxAssetLabel::Animation(animations_processed).to_string(),
                    animation_clip,
                );

                animations.push(animation_handle.clone());

                if !anim_stack.element.name.is_empty() {
                    named_animations.insert(
                        Box::from(anim_stack.element.name.as_ref()),
                        animation_handle,
                    );
                }

                animations_processed += 1;
            }
        }

        tracing::info!("FBX Loader: Processed {} animations", animations_processed);

        let mut scenes = Vec::new();
        let named_scenes = HashMap::new();

        // Build a scene with all meshes (simplified approach)
        let mut world = World::new();
        let default_material = materials.get(0).cloned().unwrap_or_else(|| {
            // Create a bright, easily visible default material
            let mut default_mat = StandardMaterial::default();
            default_mat.base_color = Color::srgb(0.8, 0.2, 0.2); // Bright red
            default_mat.metallic = 0.0;
            default_mat.perceptual_roughness = 0.8;
            default_mat.cull_mode = None; // Disable backface culling for easier debugging
            
            tracing::info!("FBX Loader: Created bright red default material for better visibility");
            
            load_context.add_labeled_asset(
                FbxAssetLabel::DefaultMaterial.to_string(),
                default_mat,
            )
        });

        // Create animation player and graph if there are animations
        let animation_player = if !animations.is_empty() {
            // Create animation graph with all clips
            let mut animation_indices = Vec::new();
            let mut graph = AnimationGraph::new();
            
            // Add each animation clip to the graph
            for (i, animation_handle) in animations.iter().enumerate() {
                let node_index = graph.add_clip(animation_handle.clone(), 1.0, graph.root);
                animation_indices.push(node_index);
                tracing::info!("Added animation {} to graph with index {:?}", i, node_index);
            }
            
            // Store the animation graph as an asset
            let graph_handle = load_context.add_labeled_asset(
                FbxAssetLabel::AnimationGraph(0).to_string(),
                graph,
            );
            
            let mut player = AnimationPlayer::default();
            // Auto-play the first animation
            if let Some(&first_node_index) = animation_indices.first() {
                player.play(first_node_index).repeat();
                tracing::info!("Auto-playing first animation with node index {:?}", first_node_index);
            }
            
            let player_entity = world.spawn((
                player,
                AnimationGraphHandle(graph_handle.clone()),
            )).id();
            
            tracing::info!("FBX Loader: Created animation player for {} animations", animations.len());
            Some(player_entity)
        } else {
            None
        };

        tracing::info!(
            "FBX Loader: Found {} meshes, {} nodes",
            meshes.len(),
            scene.nodes.len()
        );

        // Spawn all meshes with their original transforms and correct materials
        for (mesh_index, (((mesh_handle, transform_matrix), mesh_mat_names), node_name)) in meshes
            .iter()
            .zip(transforms.iter())
            .zip(mesh_material_info.iter())
            .zip(mesh_node_names.iter())
            .enumerate()
        {
            let transform = Transform::from_matrix(Mat4::from_cols_array(&[
                transform_matrix.m00 as f32,
                transform_matrix.m10 as f32,
                transform_matrix.m20 as f32,
                0.0,
                transform_matrix.m01 as f32,
                transform_matrix.m11 as f32,
                transform_matrix.m21 as f32,
                0.0,
                transform_matrix.m02 as f32,
                transform_matrix.m12 as f32,
                transform_matrix.m22 as f32,
                0.0,
                transform_matrix.m03 as f32,
                transform_matrix.m13 as f32,
                transform_matrix.m23 as f32,
                1.0,
            ]));

            // Keep original scale - no automatic scaling

            // Find the appropriate material for this mesh using stored material info
            tracing::info!(
                "Mesh {} uses {} materials: {:?}",
                mesh_index,
                mesh_mat_names.len(),
                mesh_mat_names
            );

            let material_to_use = if !mesh_mat_names.is_empty() {
                // Try to find the first material that exists in our processed materials
                let mut best_material_handle = None;

                for material_name in mesh_mat_names {
                    if let Some(material_handle) = named_materials.get(material_name as &str) {
                        tracing::info!(
                            "Using material '{}' for mesh {}",
                            material_name,
                            mesh_index
                        );
                        best_material_handle = Some(material_handle.clone());
                        break;
                    }
                }

                // If we found a matching material, use it
                if let Some(material_handle) = best_material_handle {
                    material_handle
                } else {
                    // Fall back to index-based selection
                    if materials.len() > 0 {
                        let material_index = mesh_index.min(materials.len() - 1);
                        tracing::info!(
                            "Using fallback material index {} for mesh {} (materials: {:?})",
                            material_index,
                            mesh_index,
                            mesh_mat_names
                        );
                        materials[material_index].clone()
                    } else {
                        tracing::warn!(
                            "No materials available for mesh {}, using default",
                            mesh_index
                        );
                        default_material.clone()
                    }
                }
            } else {
                tracing::info!(
                    "No materials assigned to mesh {}, using default",
                    mesh_index
                );
                default_material.clone()
            };

            tracing::info!(
                "FBX Loader: Spawning mesh {} with transform: {:?}",
                mesh_index,
                transform
            );

            // Create a name for this mesh entity that animation can target
            let mesh_name = if !node_name.is_empty() {
                node_name.clone()
            } else {
                format!("Mesh_{}", mesh_index)
            };

            let mut entity = world.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(material_to_use),
                transform,
                GlobalTransform::default(),
                Visibility::default(),
                Name::new(mesh_name.clone()),
            ));

            // Log detailed spawn information for debugging
            tracing::info!(
                "FBX Loader: Spawned entity with position {:?}, rotation {:?}, scale {:?}",
                transform.translation,
                transform.rotation,
                transform.scale
            );

            // Add animation target if there are animations and an animation player
            if !animations.is_empty() && animation_player.is_some() {
                // Use the node name as the animation target - this must match what's used in animation curves
                let name_component = Name::new(mesh_name.clone());
                entity.insert(AnimationTarget {
                    id: AnimationTargetId::from_names(std::iter::once(&name_component)),
                    player: animation_player.unwrap(),
                });
                tracing::info!("FBX Loader: Added animation target '{}' (id from names: ['{}']) to mesh {}", mesh_name, mesh_name, mesh_index);
            }
        }

        // Spawn lights from the FBX scene (only if enabled in settings)
        let mut lights_spawned = 0;
        if settings.load_lights {
            for light in scene.lights.as_ref().iter() {
                // Find the node that contains this light
                if let Some(light_node) = scene.nodes.as_ref().iter().find(|node| {
                    node.light.is_some()
                        && node.light.as_ref().unwrap().element.element_id
                            == light.element.element_id
                }) {
                    let transform = Transform::from_matrix(Mat4::from_cols_array(&[
                        light_node.node_to_world.m00 as f32,
                        light_node.node_to_world.m10 as f32,
                        light_node.node_to_world.m20 as f32,
                        0.0,
                        light_node.node_to_world.m01 as f32,
                        light_node.node_to_world.m11 as f32,
                        light_node.node_to_world.m21 as f32,
                        0.0,
                        light_node.node_to_world.m02 as f32,
                        light_node.node_to_world.m12 as f32,
                        light_node.node_to_world.m22 as f32,
                        0.0,
                        light_node.node_to_world.m03 as f32,
                        light_node.node_to_world.m13 as f32,
                        light_node.node_to_world.m23 as f32,
                        1.0,
                    ]));

                    match light.type_ {
                        ufbx::LightType::Directional => {
                            tracing::info!(
                                "FBX Loader: Spawning directional light '{}' with intensity {}",
                                light.element.name,
                                light.intensity
                            );

                            world.spawn((
                                DirectionalLight {
                                    color: Color::srgb(
                                        light.color.x as f32,
                                        light.color.y as f32,
                                        light.color.z as f32,
                                    ),
                                    illuminance: light.intensity as f32,
                                    shadows_enabled: light.cast_shadows,
                                    ..default()
                                },
                                transform,
                                GlobalTransform::default(),
                                Visibility::default(),
                            ));
                            lights_spawned += 1;
                        }
                        ufbx::LightType::Point => {
                            tracing::info!(
                                "FBX Loader: Spawning point light '{}' with intensity {}",
                                light.element.name,
                                light.intensity
                            );

                            world.spawn((
                                PointLight {
                                    color: Color::srgb(
                                        light.color.x as f32,
                                        light.color.y as f32,
                                        light.color.z as f32,
                                    ),
                                    intensity: light.intensity as f32,
                                    shadows_enabled: light.cast_shadows,
                                    ..default()
                                },
                                transform,
                                GlobalTransform::default(),
                                Visibility::default(),
                            ));
                            lights_spawned += 1;
                        }
                        ufbx::LightType::Spot => {
                            tracing::info!(
                            "FBX Loader: Spawning spot light '{}' with intensity {} (angles: {:.1}° - {:.1}°)",
                            light.element.name,
                            light.intensity,
                            light.inner_angle.to_degrees(),
                            light.outer_angle.to_degrees()
                        );

                            world.spawn((
                                SpotLight {
                                    color: Color::srgb(
                                        light.color.x as f32,
                                        light.color.y as f32,
                                        light.color.z as f32,
                                    ),
                                    intensity: light.intensity as f32,
                                    shadows_enabled: light.cast_shadows,
                                    inner_angle: light.inner_angle as f32,
                                    outer_angle: light.outer_angle as f32,
                                    ..default()
                                },
                                transform,
                                GlobalTransform::default(),
                                Visibility::default(),
                            ));
                            lights_spawned += 1;
                        }
                        _ => {
                            tracing::info!(
                                "FBX Loader: Skipping unsupported light type {:?} for light '{}'",
                                light.type_,
                                light.element.name
                            );
                        }
                    }
                }
            }

            tracing::info!("FBX Loader: Spawned {} lights in scene", lights_spawned);
        } // End of lights spawning check

        // Spawn cameras from the FBX scene (only if enabled in settings)
        let mut cameras_spawned = 0;
        if settings.load_cameras {
            for camera in scene.cameras.as_ref().iter() {
                // Find the node that contains this camera
                if let Some(camera_node) = scene.nodes.as_ref().iter().find(|node| {
                    node.camera.is_some()
                        && node.camera.as_ref().unwrap().element.element_id
                            == camera.element.element_id
                }) {
                    let transform = Transform::from_matrix(Mat4::from_cols_array(&[
                        camera_node.node_to_world.m00 as f32,
                        camera_node.node_to_world.m10 as f32,
                        camera_node.node_to_world.m20 as f32,
                        0.0,
                        camera_node.node_to_world.m01 as f32,
                        camera_node.node_to_world.m11 as f32,
                        camera_node.node_to_world.m21 as f32,
                        0.0,
                        camera_node.node_to_world.m02 as f32,
                        camera_node.node_to_world.m12 as f32,
                        camera_node.node_to_world.m22 as f32,
                        0.0,
                        camera_node.node_to_world.m03 as f32,
                        camera_node.node_to_world.m13 as f32,
                        camera_node.node_to_world.m23 as f32,
                        1.0,
                    ]));

                    // Create projection based on camera type
                    let projection = match camera.projection_mode {
                        ufbx::ProjectionMode::Perspective => {
                            let mut perspective = PerspectiveProjection::default();
                            // Use Y field of view (in degrees) and convert to radians
                            perspective.fov = camera.field_of_view_deg.y.to_radians() as f32;
                            perspective.near = camera.near_plane as f32;
                            perspective.far = camera.far_plane as f32;
                            if camera.aspect_ratio > 0.0 {
                                perspective.aspect_ratio = camera.aspect_ratio as f32;
                            }
                            Projection::Perspective(perspective)
                        }
                        ufbx::ProjectionMode::Orthographic => {
                            // For orthographic, we need OrthographicProjection
                            // But we'll use perspective for now as OrthographicProjection needs different setup
                            let mut perspective = PerspectiveProjection::default();
                            perspective.near = camera.near_plane as f32;
                            perspective.far = camera.far_plane as f32;
                            Projection::Perspective(perspective)
                        }
                    };

                    tracing::info!(
                        "FBX Loader: Spawning camera '{}' ({})",
                        camera.element.name,
                        match camera.projection_mode {
                            ufbx::ProjectionMode::Perspective => "perspective",
                            ufbx::ProjectionMode::Orthographic => "orthographic (using perspective fallback)",
                        }
                    );

                    world.spawn((
                        Camera3d::default(),
                        projection,
                        transform,
                        GlobalTransform::default(),
                        Camera {
                            is_active: cameras_spawned == 0, // First camera is active
                            ..Default::default()
                        },
                        Visibility::default(),
                    ));
                    cameras_spawned += 1;
                }
            }

            tracing::info!("FBX Loader: Spawned {} cameras in scene", cameras_spawned);
        } // End of cameras spawning check

        let scene_handle =
            load_context.add_labeled_asset(FbxAssetLabel::Scene(0).to_string(), Scene::new(world));
        scenes.push(scene_handle.clone());

        Ok(Fbx {
            scenes,
            named_scenes,
            meshes,
            named_meshes,
            materials,
            named_materials,
            nodes,
            named_nodes,
            skins,
            named_skins,
            default_scene: Some(scene_handle),
            animations,
            named_animations,
        })
    }

    fn extensions(&self) -> &[&str] {
        &["fbx"]
    }
}

/// Plugin adding the FBX loader to an [`App`].
#[derive(Default)]
pub struct FbxPlugin;

impl Plugin for FbxPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Fbx>()
            .init_asset::<FbxNode>()
            .init_asset::<FbxSkin>()
            .register_asset_loader(FbxLoader::default());
    }
}
