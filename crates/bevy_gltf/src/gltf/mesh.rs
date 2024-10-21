use core::ops::Deref;

use bevy_pbr::StandardMaterial;
use serde::Deserialize;

use bevy_asset::{Asset, Handle, LoadContext, RenderAssetUsages};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Vec3;
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_render::mesh::{
    morph::{MorphAttributes, MorphTargetImage},
    Indices, Mesh, PrimitiveTopology, VertexAttributeValues,
};
use bevy_utils::{
    tracing::{error, info_span, warn},
    HashMap,
};

use crate::{
    ext::{ExtrasExt, GltfExt, MaterialExt, ModeExt},
    vertex_attributes::convert_attribute,
    GltfError, GltfLoader, GltfLoaderSettings,
};

use super::{GltfAssetLabel, GltfExtras, GltfPrimitive};

/// A glTF mesh, which may consist of multiple [`GltfPrimitives`](GltfPrimitive)
/// and an optional [`GltfExtras`].
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-mesh).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GltfMesh {
    /// Index of the mesh inside the scene
    pub index: usize,
    /// Computed name for a mesh - either a user defined mesh name from gLTF or a generated name from index
    pub name: String,
    /// Primitives of the glTF mesh.
    pub primitives: Vec<GltfPrimitive>,
    /// Additional data.
    pub extras: Option<GltfExtras>,
}

impl GltfMesh {
    /// Create a mesh extracting name and index from glTF def
    pub fn new(
        mesh: &gltf::Mesh,
        primitives: Vec<GltfPrimitive>,
        extras: Option<GltfExtras>,
    ) -> Self {
        Self {
            index: mesh.index(),
            name: if let Some(name) = mesh.name() {
                name.to_string()
            } else {
                format!("GltfMesh{}", mesh.index())
            },
            primitives,
            extras,
        }
    }

    #[allow(clippy::result_large_err, clippy::too_many_arguments)]
    /// Load all meshes of a [`glTF`](gltf::Gltf)
    pub(crate) fn load_meshes(
        loader: &GltfLoader,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        file_name: &str,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
        materials: &[Handle<StandardMaterial>],
    ) -> Result<(Vec<Handle<GltfMesh>>, HashMap<Box<str>, Handle<GltfMesh>>), GltfError> {
        let (meshes_on_skinned_nodes, meshes_on_non_skinned_nodes) = gltf.load_meshes_on_nodes();

        let mut meshes = vec![];
        let mut named_meshes = HashMap::default();
        for gltf_mesh in gltf.meshes() {
            let mut primitives = vec![];
            for primitive in gltf_mesh.primitives() {
                let primitive_label = GltfAssetLabel::Primitive {
                    mesh: gltf_mesh.index(),
                    primitive: primitive.index(),
                };
                let primitive_topology = primitive.mode().get_primitive_topology()?;

                let mut mesh = Mesh::new(primitive_topology, settings.load_meshes);

                // Read vertex attributes
                for (semantic, accessor) in primitive.attributes() {
                    if [
                        gltf::mesh::Semantic::Joints(0),
                        gltf::mesh::Semantic::Weights(0),
                    ]
                    .contains(&semantic)
                    {
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
                        buffer_data,
                        &loader.custom_vertex_attributes,
                    ) {
                        Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
                        Err(err) => warn!("{}", err),
                    }
                }

                // Read vertex indices
                let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].deref()));
                if let Some(indices) = reader.read_indices() {
                    mesh.insert_indices(match indices {
                        gltf::mesh::util::ReadIndices::U8(is) => {
                            Indices::U16(is.map(|x| x as u16).collect())
                        }
                        gltf::mesh::util::ReadIndices::U16(is) => Indices::U16(is.collect()),
                        gltf::mesh::util::ReadIndices::U32(is) => Indices::U32(is.collect()),
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
                        let handle = load_context.add_labeled_asset(
                            morph_targets_label.to_string(),
                            morph_target_image.0,
                        );

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
                    && primitive.material().needs_tangents()
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
                primitives.push(GltfPrimitive::new(
                    &gltf_mesh,
                    &primitive,
                    mesh_handle,
                    primitive
                        .material()
                        .index()
                        .and_then(|i| materials.get(i).cloned()),
                    primitive.extras().get(),
                    primitive.material().extras().get(),
                ));
            }

            let mesh = GltfMesh::new(&gltf_mesh, primitives, gltf_mesh.extras().get());

            let handle = load_context.add_labeled_asset(mesh.asset_label().to_string(), mesh);
            if let Some(name) = gltf_mesh.name() {
                named_meshes.insert(name.into(), handle.clone());
            }
            meshes.push(handle);
        }

        Ok((meshes, named_meshes))
    }

    /// Subasset label for this mesh within the gLTF parent asset.
    pub fn asset_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Mesh(self.index)
    }
}

/// Additional untyped data that can be present on most glTF types at the mesh level.
///
/// See [the relevant glTF specification section](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-extras).
#[derive(Clone, Debug, Reflect, Default, Component)]
#[reflect(Component, Default, Debug)]
pub struct GltfMeshExtras {
    /// Content of the extra data.
    pub value: String,
}

pub(super) struct PrimitiveMorphAttributesIter<'s>(
    pub  (
        Option<gltf::accessor::Iter<'s, [f32; 3]>>,
        Option<gltf::accessor::Iter<'s, [f32; 3]>>,
        Option<gltf::accessor::Iter<'s, [f32; 3]>>,
    ),
);

impl<'s> Iterator for PrimitiveMorphAttributesIter<'s> {
    type Item = MorphAttributes;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.0 .0.as_mut().and_then(Iterator::next);
        let normal = self.0 .1.as_mut().and_then(Iterator::next);
        let tangent = self.0 .2.as_mut().and_then(Iterator::next);
        if position.is_none() && normal.is_none() && tangent.is_none() {
            return None;
        }

        Some(MorphAttributes {
            position: position.map(Into::into).unwrap_or(Vec3::ZERO),
            normal: normal.map(Into::into).unwrap_or(Vec3::ZERO),
            tangent: tangent.map(Into::into).unwrap_or(Vec3::ZERO),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MorphTargetNames {
    pub target_names: Vec<String>,
}
