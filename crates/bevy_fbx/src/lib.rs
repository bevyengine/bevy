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
use bevy_mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues};
use bevy_mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_pbr::{MeshMaterial3d, StandardMaterial};

use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_render::mesh::Mesh3d;
use bevy_render::prelude::Visibility;
use bevy_scene::Scene;

use bevy_animation::AnimationClip;
use bevy_transform::prelude::*;
use bevy_math::{Mat4, Vec3, Quat};
use bevy_color::Color;
use bevy_image::Image;
use bevy_render::alpha::AlphaMode;

mod label;
pub use label::FbxAssetLabel;

pub mod prelude {
    //! Commonly used items.
    pub use crate::{Fbx, FbxAssetLabel, FbxPlugin};
}

/// Type of relationship between two objects in the FBX hierarchy.
#[derive(Debug, Clone)]
pub enum FbxConnKind {
    /// Standard parent-child connection.
    Parent,
    /// Connection from an object to one of its properties.
    ObjectProperty,
    /// Constraint relationship.
    Constraint,
}

/// Simplified connection entry extracted from the FBX file.
#[derive(Debug, Clone)]
pub struct FbxConnection {
    /// Source object identifier.
    pub src: String,
    /// Destination object identifier.
    pub dst: String,
    /// The type of this connection.
    pub kind: FbxConnKind,
}

/// Handedness of a coordinate system.
#[derive(Debug, Clone, Copy)]
pub enum Handedness {
    /// Right handed coordinate system.
    Right,
    /// Left handed coordinate system.
    Left,
}

/// Coordinate axes definition stored in an FBX file.
#[derive(Debug, Clone, Copy)]
pub struct FbxAxisSystem {
    /// Up axis.
    pub up: Vec3,
    /// Forward axis.
    pub front: Vec3,
    /// Coordinate system handedness.
    pub handedness: Handedness,
}

/// Metadata found in the FBX header.
#[derive(Debug, Clone)]
pub struct FbxMeta {
    /// Creator string.
    pub creator: Option<String>,
    /// Timestamp when the file was created.
    pub creation_time: Option<String>,
    /// Original application that generated the file.
    pub original_application: Option<String>,
}

/// Placeholder type for skeleton data.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct Skeleton;

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

/// Texture wrapping modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxWrapMode {
    /// Repeat the texture.
    Repeat,
    /// Clamp to edge.
    Clamp,
}

/// Texture information from FBX.
#[derive(Debug, Clone)]
pub struct FbxTexture {
    /// Texture name.
    pub name: String,
    /// Relative filename.
    pub filename: String,
    /// Absolute filename if available.
    pub absolute_filename: String,
    /// UV set name.
    pub uv_set: String,
    /// UV transformation matrix.
    pub uv_transform: Mat4,
    /// U-axis wrapping mode.
    pub wrap_u: FbxWrapMode,
    /// V-axis wrapping mode.
    pub wrap_v: FbxWrapMode,
}

/// Enhanced material representation from FBX.
#[derive(Debug, Clone)]
pub struct FbxMaterial {
    /// Material name.
    pub name: String,
    /// Base color (albedo).
    pub base_color: Color,
    /// Metallic factor.
    pub metallic: f32,
    /// Roughness factor.
    pub roughness: f32,
    /// Emission color.
    pub emission: Color,
    /// Normal map scale.
    pub normal_scale: f32,
    /// Alpha value.
    pub alpha: f32,
    /// Associated textures.
    pub textures: HashMap<FbxTextureType, FbxTexture>,
}

/// Types of lights supported in FBX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxLightType {
    /// Directional light.
    Directional,
    /// Point light.
    Point,
    /// Spot light with cone.
    Spot,
    /// Area light.
    Area,
    /// Volume light.
    Volume,
}

/// Light definition from FBX.
#[derive(Debug, Clone)]
pub struct FbxLight {
    /// Light name.
    pub name: String,
    /// Light type.
    pub light_type: FbxLightType,
    /// Light color.
    pub color: Color,
    /// Light intensity.
    pub intensity: f32,
    /// Whether the light casts shadows.
    pub cast_shadows: bool,
    /// Inner cone angle for spot lights (degrees).
    pub inner_angle: Option<f32>,
    /// Outer cone angle for spot lights (degrees).
    pub outer_angle: Option<f32>,
}

/// Camera projection modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxProjectionMode {
    /// Perspective projection.
    Perspective,
    /// Orthographic projection.
    Orthographic,
}

/// Camera definition from FBX.
#[derive(Debug, Clone)]
pub struct FbxCamera {
    /// Camera name.
    pub name: String,
    /// Projection mode.
    pub projection_mode: FbxProjectionMode,
    /// Field of view in degrees.
    pub field_of_view_deg: f32,
    /// Aspect ratio.
    pub aspect_ratio: f32,
    /// Near clipping plane.
    pub near_plane: f32,
    /// Far clipping plane.
    pub far_plane: f32,
    /// Focal length in millimeters.
    pub focal_length_mm: f32,
}

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

/// Animation stack representing a timeline.
#[derive(Debug, Clone)]
pub struct FbxAnimStack {
    /// Animation stack name.
    pub name: String,
    /// Start time in seconds.
    pub time_begin: f64,
    /// End time in seconds.
    pub time_end: f64,
    /// Animation layers in this stack.
    pub layers: Vec<FbxAnimLayer>,
}

/// Animation layer within a stack.
#[derive(Debug, Clone)]
pub struct FbxAnimLayer {
    /// Layer name.
    pub name: String,
    /// Layer weight.
    pub weight: f32,
    /// Whether this layer is additive.
    pub additive: bool,
    /// Property animations in this layer.
    pub property_animations: Vec<FbxPropertyAnim>,
}

/// Property animation data.
#[derive(Debug, Clone)]
pub struct FbxPropertyAnim {
    /// Target node ID.
    pub node_id: u32,
    /// Property name (e.g., "Lcl Translation", "Lcl Rotation").
    pub property: String,
    /// Animation curves for each component.
    pub curves: Vec<FbxAnimCurve>,
}

/// Animation curve data.
#[derive(Debug, Clone)]
pub struct FbxAnimCurve {
    /// Keyframe times.
    pub times: Vec<f64>,
    /// Keyframe values.
    pub values: Vec<f32>,
    /// Interpolation mode.
    pub interpolation: FbxInterpolation,
}

/// Animation interpolation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FbxInterpolation {
    /// Constant interpolation.
    Constant,
    /// Linear interpolation.
    Linear,
    /// Cubic interpolation.
    Cubic,
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
    /// Original axis system of the file.
    pub axis_system: FbxAxisSystem,
    /// Conversion factor from the original unit to meters.
    pub unit_scale: f32,
    /// Copyright, creator and tool information.
    pub metadata: FbxMeta,
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

/// Loader implementation for FBX files.
#[derive(Default)]
pub struct FbxLoader;

impl AssetLoader for FbxLoader {
    type Asset = Fbx;
    type Settings = ();
    type Error = FbxError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
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
            return Err(FbxError::Parse("FBX file too small to be valid".to_string()));
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

        let mut meshes = Vec::new();
        let mut named_meshes = HashMap::new();
        let mut transforms = Vec::new();
        let mut scratch = Vec::new();

        for (index, node) in scene.nodes.as_ref().iter().enumerate() {
            let Some(mesh_ref) = node.mesh.as_ref() else { continue };
            let mesh = mesh_ref.as_ref();

            // Basic mesh validation
            if mesh.num_vertices == 0 || mesh.faces.as_ref().is_empty() {
                continue;
            }

            // Each mesh becomes a Bevy `Mesh` asset.
            let handle =
                load_context.labeled_asset_scope::<_, FbxError>(FbxAssetLabel::Mesh(index).to_string(), |_lc| {
                    let positions: Vec<[f32; 3]> = mesh
                        .vertex_position
                        .values
                        .as_ref()
                        .iter()
                        .map(|v| [v.x as f32, v.y as f32, v.z as f32])
                        .collect();

                    let mut bevy_mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::default(),
                    );
                    bevy_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

                    if mesh.vertex_normal.exists {
                        let normals: Vec<[f32; 3]> = (0..mesh.num_vertices)
                            .map(|i| {
                                let n = mesh.vertex_normal[i];
                                [n.x as f32, n.y as f32, n.z as f32]
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

                            for (cluster_index, cluster) in skin_deformer.clusters.iter().enumerate() {
                                if weight_count >= 4 { break; }

                                // Find weight for this vertex in this cluster
                                for &weight_vertex in cluster.vertices.iter() {
                                    if weight_vertex as usize == vertex_index {
                                        if let Some(weight_index) = cluster.vertices.iter().position(|&v| v as usize == vertex_index) {
                                            if weight_index < cluster.weights.len() {
                                                let weight = cluster.weights[weight_index] as f32;
                                                if weight > 0.0 {
                                                    joint_indices[vertex_index][weight_count] = cluster_index as u16;
                                                    joint_weights[vertex_index][weight_count] = weight;
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
                        bevy_mesh.insert_attribute(
                            Mesh::ATTRIBUTE_JOINT_WEIGHT,
                            joint_weights,
                        );
                    }

                    let mut indices = Vec::new();
                    for &face in mesh.faces.as_ref() {
                        scratch.clear();
                        ufbx::triangulate_face_vec(&mut scratch, mesh, face);
                        for idx in &scratch {
                            let v = mesh.vertex_indices[*idx as usize];
                            indices.push(v);
                        }
                    }
                    bevy_mesh.insert_indices(Indices::U32(indices));

                    Ok(bevy_mesh)
                })?;
            if !node.element.name.is_empty() {
                named_meshes.insert(Box::from(node.element.name.as_ref()), handle.clone());
            }
            meshes.push(handle);
            transforms.push(node.geometry_to_world);
        }

        // Process textures and materials
        let mut fbx_textures = Vec::new();
        let mut texture_handles = HashMap::new();

        // First pass: collect all textures
        for texture in scene.textures.as_ref().iter() {
            let fbx_texture = FbxTexture {
                name: texture.element.name.to_string(),
                filename: texture.filename.to_string(),
                absolute_filename: texture.absolute_filename.to_string(),
                uv_set: texture.uv_set.to_string(),
                uv_transform: Mat4::IDENTITY, // Note: UV transform conversion not implemented yet
                wrap_u: match texture.wrap_u {
                    ufbx::WrapMode::Repeat => FbxWrapMode::Repeat,
                    _ => FbxWrapMode::Clamp,
                },
                wrap_v: match texture.wrap_v {
                    ufbx::WrapMode::Repeat => FbxWrapMode::Repeat,
                    _ => FbxWrapMode::Clamp,
                },
            };

            // Try to load the texture file
            if !texture.filename.is_empty() {
                let texture_path = if !texture.absolute_filename.is_empty() {
                    texture.absolute_filename.to_string()
                } else {
                    // Try relative to the FBX file
                    let fbx_dir = load_context.path().parent().unwrap_or_else(|| std::path::Path::new(""));
                    fbx_dir.join(texture.filename.as_ref()).to_string_lossy().to_string()
                };

                // Load texture as Image asset
                let image_handle: Handle<Image> = load_context.load(texture_path);
                texture_handles.insert(texture.element.element_id, image_handle);
            }

            fbx_textures.push(fbx_texture);
        }

        // Convert materials with enhanced PBR support
        let mut materials = Vec::new();
        let mut named_materials = HashMap::new();
        let mut fbx_materials = Vec::new();

        for (index, ufbx_material) in scene.materials.as_ref().iter().enumerate() {
            // Extract material properties
            let mut base_color = Color::srgb(1.0, 1.0, 1.0);
            let mut metallic = 0.0f32;
            let mut roughness = 0.5f32;
            let mut emission = Color::BLACK;
            let mut normal_scale = 1.0f32;
            let mut alpha = 1.0f32;
            let mut material_textures = HashMap::new();

            // Note: Advanced material property extraction not implemented yet
            // Using default PBR values for now
            roughness = 0.5f32;

            // Note: Texture processing not fully implemented yet
            // Basic texture loading is supported but not applied to materials

            let fbx_material = FbxMaterial {
                name: ufbx_material.element.name.to_string(),
                base_color,
                metallic,
                roughness,
                emission,
                normal_scale,
                alpha,
                textures: material_textures,
            };

            // Create StandardMaterial with textures
            let mut standard_material = StandardMaterial {
                base_color: fbx_material.base_color,
                metallic: fbx_material.metallic,
                perceptual_roughness: fbx_material.roughness,
                emissive: fbx_material.emission.into(),
                alpha_mode: if fbx_material.alpha < 1.0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Opaque
                },
                ..Default::default()
            };

            // Note: Texture application to materials not implemented yet
            // Textures are loaded but not yet applied to StandardMaterial

            let handle = load_context.add_labeled_asset(
                FbxAssetLabel::Material(index).to_string(),
                standard_material,
            );

            if !ufbx_material.element.name.is_empty() {
                named_materials.insert(Box::from(ufbx_material.element.name.as_ref()), handle.clone());
            }

            fbx_materials.push(fbx_material);
            materials.push(handle);
        }

        // Process skins first
        let mut skins = Vec::new();
        let mut named_skins = HashMap::new();
        let mut skin_map = HashMap::new(); // Map from ufbx skin ID to FbxSkin handle

        for (skin_index, mesh_node) in scene.nodes.as_ref().iter().enumerate() {
            let Some(mesh_ref) = &mesh_node.mesh else { continue };
            let mesh = mesh_ref.as_ref();

            if mesh.skin_deformers.is_empty() { continue; }

            let skin_deformer = &mesh.skin_deformers[0];

            // Create inverse bind matrices
            let mut inverse_bind_matrices = Vec::new();
            let mut joint_node_ids = Vec::new();

            for cluster in &skin_deformer.clusters {
                // Convert ufbx matrix to Mat4
                let bind_matrix = cluster.bind_to_world;
                let inverse_bind_matrix = Mat4::from_cols_array(&[
                    bind_matrix.m00 as f32, bind_matrix.m10 as f32, bind_matrix.m20 as f32, 0.0,
                    bind_matrix.m01 as f32, bind_matrix.m11 as f32, bind_matrix.m21 as f32, 0.0,
                    bind_matrix.m02 as f32, bind_matrix.m12 as f32, bind_matrix.m22 as f32, 0.0,
                    bind_matrix.m03 as f32, bind_matrix.m13 as f32, bind_matrix.m23 as f32, 1.0,
                ]).inverse();

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
                skin_map.insert(mesh_node.element.element_id, (inverse_bindposes_handle, joint_node_ids, skin_name, skin_index));
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
                meshes.iter().enumerate().find_map(|(mesh_idx, mesh_handle)| {
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

            let node_handle = load_context.add_labeled_asset(
                FbxAssetLabel::Node(index).to_string(),
                fbx_node,
            );

            node_map.insert(ufbx_node.element.element_id, node_handle.clone());
            nodes.push(node_handle.clone());

            if !ufbx_node.element.name.is_empty() {
                named_nodes.insert(Box::from(ufbx_node.element.name.as_ref()), node_handle);
            }
        }

        // Second pass: establish parent-child relationships
        // Note: We skip this for now to avoid ufbx crashes with children access
        // Note: Parent-child relationships not implemented yet to avoid ufbx crashes

        // Third pass: Create actual FbxSkin assets now that all nodes are created
        for (_mesh_node_id, (inverse_bindposes_handle, joint_node_ids, skin_name, skin_index)) in skin_map.iter() {
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

            let skin_handle = load_context.add_labeled_asset(
                FbxAssetLabel::Skin(*skin_index).to_string(),
                fbx_skin,
            );

            skins.push(skin_handle.clone());

            if !skin_name.starts_with("Skin_") {
                named_skins.insert(Box::from(skin_name.as_str()), skin_handle);
            }
        }

        // Process animations (simplified for now)
        let animations = Vec::new();
        let named_animations = HashMap::new();

        // Note: Full animation processing not implemented yet
        // Basic structure is in place but needs ufbx animation API integration

        let mut scenes = Vec::new();
        let named_scenes = HashMap::new();

        // Build a scene with all meshes (simplified approach)
        let mut world = World::new();
        let default_material = materials.get(0).cloned().unwrap_or_else(|| {
            load_context.add_labeled_asset(
                FbxAssetLabel::DefaultMaterial.to_string(),
                StandardMaterial::default(),
            )
        });

        tracing::info!("FBX Loader: Found {} meshes, {} nodes", meshes.len(), scene.nodes.len());

        // For now, spawn all meshes with their original transforms
        for (mesh_index, (mesh_handle, transform_matrix)) in meshes.iter().zip(transforms.iter()).enumerate() {
            let transform = Transform::from_matrix(Mat4::from_cols_array(&[
                transform_matrix.m00 as f32, transform_matrix.m10 as f32, transform_matrix.m20 as f32, 0.0,
                transform_matrix.m01 as f32, transform_matrix.m11 as f32, transform_matrix.m21 as f32, 0.0,
                transform_matrix.m02 as f32, transform_matrix.m12 as f32, transform_matrix.m22 as f32, 0.0,
                transform_matrix.m03 as f32, transform_matrix.m13 as f32, transform_matrix.m23 as f32, 1.0,
            ]));

            tracing::info!("FBX Loader: Spawning mesh {} with transform: {:?}", mesh_index, transform);

            world.spawn((
                Mesh3d(mesh_handle.clone()),
                MeshMaterial3d(default_material.clone()),
                transform,
                GlobalTransform::default(),
                Visibility::default(),
            ));
        }

        let scene_handle = load_context.add_labeled_asset(FbxAssetLabel::Scene(0).to_string(), Scene::new(world));
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
            // Note: Using default axis system (matches Bevy's coordinate system)
            axis_system: FbxAxisSystem {
                up: Vec3::Y,
                front: Vec3::Z,
                handedness: Handedness::Right,
            },
            // Note: Unit scale is handled by ufbx target_unit_meters setting
            unit_scale: 1.0,
            // Note: Metadata extraction not implemented yet
            metadata: FbxMeta {
                creator: None,
                creation_time: None,
                original_application: None,
            },
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
            .init_asset::<Skeleton>()
            .register_asset_loader(FbxLoader::default());
    }
}
