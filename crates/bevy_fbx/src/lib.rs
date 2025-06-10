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
use bevy_mesh::{Indices, Mesh, PrimitiveTopology};
use bevy_pbr::{MeshMaterial3d, StandardMaterial};

use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use bevy_render::mesh::Mesh3d;
use bevy_render::prelude::Visibility;
use bevy_scene::Scene;

use bevy_animation::AnimationClip;
use bevy_transform::prelude::*;
use bevy_math::{Mat4, Vec3};
use bevy_color::Color;

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
    pub inverse_bind_matrices: Handle<bevy_mesh::skinning::SkinnedMeshInverseBindposes>,
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

        // Convert materials. Currently these are simple placeholders.
        let mut materials = Vec::new();
        let mut named_materials = HashMap::new();
        for (index, mat) in scene.materials.as_ref().iter().enumerate() {
            let handle = load_context.add_labeled_asset(
                FbxAssetLabel::Material(index).to_string(),
                StandardMaterial::default(),
            );
            if !mat.element.name.is_empty() {
                named_materials.insert(Box::from(mat.element.name.as_ref()), handle.clone());
            }
            materials.push(handle);
        }

        // Build nodes and scenes
        let nodes = Vec::new();
        let named_nodes = HashMap::new();
        let mut scenes = Vec::new();
        let named_scenes = HashMap::new();

        // Build a simple scene with all meshes
        let mut world = World::new();
        let default_material = materials.get(0).cloned().unwrap_or_else(|| {
            load_context.add_labeled_asset(
                FbxAssetLabel::DefaultMaterial.to_string(),
                StandardMaterial::default(),
            )
        });

        for (mesh_handle, matrix) in meshes.iter().zip(transforms.iter()) {
            let mat = Mat4::from_cols_array(&[
                matrix.m00 as f32, matrix.m10 as f32, matrix.m20 as f32, 0.0,
                matrix.m01 as f32, matrix.m11 as f32, matrix.m21 as f32, 0.0,
                matrix.m02 as f32, matrix.m12 as f32, matrix.m22 as f32, 0.0,
                matrix.m03 as f32, matrix.m13 as f32, matrix.m23 as f32, 1.0,
            ]);
            let transform = Transform::from_matrix(mat);
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
            skins: Vec::new(),
            named_skins: HashMap::new(),
            default_scene: Some(scene_handle),
            animations: Vec::new(),
            named_animations: HashMap::new(),
            axis_system: FbxAxisSystem {
                up: Vec3::Y,
                front: Vec3::Z,
                handedness: Handedness::Right,
            },
            unit_scale: 1.0,
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
