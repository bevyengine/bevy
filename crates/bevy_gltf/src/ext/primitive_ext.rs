use core::ops::Deref;

use bevy_asset::{Handle, LoadContext, RenderAssetUsages};
use bevy_core::Name;
use bevy_math::Vec3;
use bevy_pbr::StandardMaterial;
use bevy_render::mesh::{
    morph::{MorphAttributes, MorphTargetImage},
    Indices, Mesh, PrimitiveTopology, VertexAttributeValues,
};
use bevy_utils::{
    tracing::{error, info_span, warn},
    HashSet,
};
use serde::Deserialize;

use crate::{
    vertex_attributes::convert_attribute, GltfAssetLabel, GltfError, GltfLoader,
    GltfLoaderSettings, GltfPrimitive,
};

use super::{ExtrasExt, MaterialExt, ModeExt};

pub trait PrimitiveExt {
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    fn load_primitive(
        &self,
        load_context: &mut LoadContext,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        parent_mesh: &gltf::Mesh,
        meshes_on_skinned_nodes: &HashSet<usize>,
        meshes_on_non_skinned_nodes: &HashSet<usize>,
        materials: &[Handle<StandardMaterial>],
    ) -> Result<GltfPrimitive, GltfError>;

    /// Generate the [`Name`] for a [`Primitive`](gltf::Primitive)
    fn primitive_name(&self, parent_mesh: &gltf::Mesh) -> Name;
}

impl PrimitiveExt for gltf::Primitive<'_> {
    fn load_primitive(
        &self,
        load_context: &mut LoadContext,
        loader: &GltfLoader,
        settings: &GltfLoaderSettings,
        file_name: &str,
        buffer_data: &[Vec<u8>],
        parent_mesh: &gltf::Mesh,
        meshes_on_skinned_nodes: &HashSet<usize>,
        meshes_on_non_skinned_nodes: &HashSet<usize>,
        materials: &[Handle<StandardMaterial>],
    ) -> Result<GltfPrimitive, GltfError> {
        let primitive_label = GltfAssetLabel::Primitive {
            mesh: parent_mesh.index(),
            primitive: self.index(),
        };
        let primitive_topology = self.mode().get_primitive_topology()?;

        let mut mesh = Mesh::new(primitive_topology, settings.load_meshes);

        // Read vertex attributes
        for (semantic, accessor) in self.attributes() {
            if [
                gltf::mesh::Semantic::Joints(0),
                gltf::mesh::Semantic::Weights(0),
            ]
            .contains(&semantic)
            {
                if !meshes_on_skinned_nodes.contains(&self.index()) {
                    warn!(
            "Ignoring attribute {:?} for skinned mesh {:?} used on non skinned nodes (NODE_SKINNED_MESH_WITHOUT_SKIN)",
            semantic,
            primitive_label
        );
                    continue;
                } else if meshes_on_non_skinned_nodes.contains(&self.index()) {
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
        let reader = self.reader(|buffer| Some(buffer_data[buffer.index()].deref()));
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
                    mesh: self.index(),
                    primitive: self.index(),
                };
                let morph_target_image = MorphTargetImage::new(
                    morph_target_reader.map(PrimitiveMorphAttributesIter),
                    mesh.count_vertices(),
                    RenderAssetUsages::default(),
                )?;
                let handle = load_context
                    .add_labeled_asset(morph_targets_label.to_string(), morph_target_image.0);

                mesh.set_morph_targets(handle);
                let extras = self.extras().as_ref();
                if let Some(names) = extras
                    .and_then(|extras| serde_json::from_str::<MorphTargetNames>(extras.get()).ok())
                {
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
            && self.material().needs_tangents()
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

        Ok(GltfPrimitive::new(
            parent_mesh,
            self,
            mesh_handle,
            self.material()
                .index()
                .and_then(|i| materials.get(i).cloned()),
            self.extras().get(),
            self.material().extras().get(),
        ))
    }

    fn primitive_name(&self, parent_mesh: &gltf::Mesh) -> Name {
        let mesh_name = parent_mesh.name().unwrap_or("Mesh");
        if parent_mesh.primitives().len() > 1 {
            format!("{}.{}", mesh_name, self.index()).into()
        } else {
            mesh_name.to_string().into()
        }
    }
}
struct PrimitiveMorphAttributesIter<'s>(
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
