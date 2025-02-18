use bevy_asset::{Asset, Handle};
use bevy_pbr::StandardMaterial;
use bevy_reflect::TypePath;
use bevy_render::mesh::Mesh;

use crate::label::GltfAssetLabel;

use super::GltfExtras;

/// Part of a [`GltfMesh`](super::GltfMesh) that consists of a [`Mesh`], an optional [`StandardMaterial`] and [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh-primitive).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfPrimitive {
    /// Index of the primitive inside the mesh
    pub index: usize,
    /// Index of the parent [`GltfMesh`](super::GltfMesh) of this primitive
    pub parent_mesh_index: usize,
    /// Computed name for a primitive - either a user defined primitive name from gLTF or a generated name from index
    pub name: String,
    /// Topology to be rendered.
    pub mesh: Handle<Mesh>,
    /// Material to apply to the `mesh`.
    pub material: Option<Handle<StandardMaterial>>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
    /// Additional data of the `material`.
    pub material_extras: Option<GltfExtras>,
}

impl GltfPrimitive {
    /// Create a primitive extracting name and index from glTF def
    pub fn new(
        gltf_mesh: &gltf::Mesh,
        gltf_primitive: &gltf::Primitive,
        mesh: Handle<Mesh>,
        material: Option<Handle<StandardMaterial>>,
        extras: Option<GltfExtras>,
        material_extras: Option<GltfExtras>,
    ) -> Self {
        GltfPrimitive {
            index: gltf_primitive.index(),
            parent_mesh_index: gltf_mesh.index(),
            name: {
                let mesh_name = gltf_mesh.name().unwrap_or("Mesh");
                if gltf_mesh.primitives().len() > 1 {
                    format!("{}.{}", mesh_name, gltf_primitive.index())
                } else {
                    mesh_name.to_string()
                }
            },
            mesh,
            material,
            extras,
            material_extras,
        }
    }

    /// Subasset label for this primitive within its parent [`GltfMesh`](super::GltfMesh) within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Primitive {
            mesh: self.parent_mesh_index,
            primitive: self.index,
        }
    }
}
