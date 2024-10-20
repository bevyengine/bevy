use core::ops::Deref;

use serde::Deserialize;

use gltf::{
    accessor::Iter,
    mesh::{util::ReadIndices, Mode},
    Primitive, Semantic,
};

use bevy_asset::{Handle, LoadContext};
use bevy_math::{Mat4, Vec3};
use bevy_pbr::StandardMaterial;
use bevy_render::{
    mesh::{
        morph::{MorphAttributes, MorphTargetImage},
        skinning::SkinnedMeshInverseBindposes,
        Indices, Mesh, VertexAttributeValues,
    },
    render_asset::RenderAssetUsages,
    render_resource::PrimitiveTopology,
};
use bevy_utils::{
    tracing::{error, info_span, warn},
    HashMap, HashSet,
};

use crate::{
    vertex_attributes::convert_attribute, GltfAssetLabel, GltfBuffer, GltfMesh, GltfPrimitive,
};

use super::{GltfError, GltfLoader, GltfLoaderSettings};

pub fn load_meshes_on_nodes(gltf: &gltf::Gltf) -> (HashSet<usize>, HashSet<usize>) {
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

    (meshes_on_skinned_nodes, meshes_on_non_skinned_nodes)
}

#[allow(clippy::result_large_err, clippy::too_many_arguments)]
pub fn load_meshes(
    loader: &GltfLoader,
    load_context: &mut LoadContext,
    settings: &GltfLoaderSettings,
    file_name: &str,
    gltf: &gltf::Gltf,
    buffer_data: &[GltfBuffer],
    meshes_on_skinned_nodes: &HashSet<usize>,
    meshes_on_non_skinned_nodes: &HashSet<usize>,
    materials: &[Handle<StandardMaterial>],
) -> Result<(Vec<Handle<GltfMesh>>, HashMap<Box<str>, Handle<GltfMesh>>), GltfError> {
    let mut meshes = vec![];
    let mut named_meshes = HashMap::default();
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
                && super::material::material_needs_tangents(&primitive.material())
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
                super::extras::get_gltf_extras(primitive.extras()),
                super::extras::get_gltf_extras(primitive.material().extras()),
            ));
        }

        let mesh = GltfMesh::new(
            &gltf_mesh,
            primitives,
            super::extras::get_gltf_extras(gltf_mesh.extras()),
        );

        let handle = load_context.add_labeled_asset(mesh.asset_label().to_string(), mesh);
        if let Some(name) = gltf_mesh.name() {
            named_meshes.insert(name.into(), handle.clone());
        }
        meshes.push(handle);
    }

    Ok((meshes, named_meshes))
}

pub fn load_skinned_mesh_inverse_bindposes(
    load_context: &mut LoadContext,
    gltf: &gltf::Gltf,
    buffer_data: &[GltfBuffer],
) -> Vec<Handle<SkinnedMeshInverseBindposes>> {
    gltf.skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let local_to_bone_bind_matrices: Vec<Mat4> = reader
                .read_inverse_bind_matrices()
                .unwrap()
                .map(|mat| Mat4::from_cols_array_2d(&mat))
                .collect();

            load_context.add_labeled_asset(
                inverse_bind_matrices_label(&gltf_skin),
                SkinnedMeshInverseBindposes::from(local_to_bone_bind_matrices),
            )
        })
        .collect()
}

pub fn primitive_name(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    let mesh_name = mesh.name().unwrap_or("Mesh");
    if mesh.primitives().len() > 1 {
        format!("{}.{}", mesh_name, primitive.index())
    } else {
        mesh_name.to_string()
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

/// Return the label for the `inverseBindMatrices` of the node.
fn inverse_bind_matrices_label(skin: &gltf::Skin) -> String {
    GltfAssetLabel::InverseBindMatrices(skin.index()).to_string()
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
