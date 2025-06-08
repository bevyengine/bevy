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
use std::sync::Arc;
use bevy_animation::AnimationClip;
use bevy_transform::prelude::*;
use bevy_math::{Mat4, Vec3};

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

/// Resulting asset for an FBX file.
#[derive(Asset, Debug, TypePath)]
pub struct Fbx {
    /* ===== Core sub-asset handles ===== */
    /// Split Bevy scenes. A single FBX may contain many scenes.
    pub scenes: Vec<Handle<Scene>>,
    /// Triangulated meshes extracted from the FBX.
    pub meshes: Vec<Handle<Mesh>>,
    /// PBR materials or fallbacks converted from FBX materials.
    pub materials: Vec<Handle<StandardMaterial>>,
    /// Flattened animation takes.
    pub animations: Vec<Handle<AnimationClip>>,
    /// Skinning skeletons.
    pub skeletons: Vec<Handle<Skeleton>>,

    /* ===== Quick name lookups ===== */
    pub named_meshes: HashMap<Box<str>, Handle<Mesh>>,
    pub named_materials: HashMap<Box<str>, Handle<StandardMaterial>>,
    pub named_animations: HashMap<Box<str>, Handle<AnimationClip>>,
    pub named_skeletons: HashMap<Box<str>, Handle<Skeleton>>,

    /* ===== FBX specific info ===== */
    /// Flattened parent/child/constraint relations.
    pub connections: Vec<FbxConnection>,
    /// Original axis system of the file.
    pub axis_system: FbxAxisSystem,
    /// Conversion factor from the original unit to meters.
    pub unit_scale: f32,
    /// Copyright, creator and tool information.
    pub metadata: FbxMeta,

    /* ===== Optional original scene bytes ===== */
    #[cfg(debug_assertions)]
    pub raw_scene_bytes: Option<Arc<[u8]>>,
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

        // Build a simple scene with all meshes at the origin.
        let mut world = World::new();
        let default_material = materials.get(0).cloned().unwrap_or_else(|| {
            load_context.add_labeled_asset(
                FbxAssetLabel::DefaultMaterial.to_string(),
                StandardMaterial::default(),
            )
        });

        for (mesh_handle, matrix) in meshes.iter().zip(transforms.iter()) {
            let mat = Mat4::from_cols_array(&[
                matrix.m00 as f32,
                matrix.m10 as f32,
                matrix.m20 as f32,
                0.0,
                matrix.m01 as f32,
                matrix.m11 as f32,
                matrix.m21 as f32,
                0.0,
                matrix.m02 as f32,
                matrix.m12 as f32,
                matrix.m22 as f32,
                0.0,
                matrix.m03 as f32,
                matrix.m13 as f32,
                matrix.m23 as f32,
                1.0,
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

        Ok(Fbx {
            scenes: vec![scene_handle.clone()],
            meshes,
            materials,
            animations: Vec::new(),
            skeletons: Vec::new(),
            named_meshes,
            named_materials,
            named_animations: HashMap::new(),
            named_skeletons: HashMap::new(),
            connections: Vec::new(),
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
            #[cfg(debug_assertions)]
            raw_scene_bytes: Some(bytes.into()),
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
            .register_asset_loader(FbxLoader::default());
    }
}
