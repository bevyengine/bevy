//! `EXT_mesh_gpu_instancing` instance transforms.
//!
//! The extension gives a mesh node per-instance `TRANSLATION`, `ROTATION` and
//! `SCALE` accessors. Each instance transform applies in the node's local
//! space, before the node's own transform.

use alloc::collections::BTreeMap;

use bevy_math::{Quat, Vec3, Vec4};
use bevy_transform::components::Transform;
use gltf::{
    accessor::{DataType, Dimensions, Item, Iter},
    animation::util::Rotations,
    Accessor, Document, Node,
};
use serde::Deserialize;
use tracing::warn;

use crate::loader::{GltfError, EXT_MESH_GPU_INSTANCING};

/// The `EXT_mesh_gpu_instancing` extension object on a node.
#[derive(Deserialize)]
struct NodeExtension {
    /// Accessor indices keyed by attribute name.
    attributes: BTreeMap<String, usize>,
}

/// Reads the instance transforms of `node`.
///
/// Returns an empty `Vec` if `node` has no mesh or no `EXT_mesh_gpu_instancing`
/// extension. Missing transform attributes default to the identity. Custom
/// attributes such as `_ID` count towards the instance count but are not
/// read; extension handlers can find their accessors in the node's extension
/// data.
///
/// Malformed extension data returns [`GltfError::InvalidMeshGpuInstancing`].
pub(crate) fn instance_transforms(
    document: &Document,
    node: &Node,
    buffer_data: &[Vec<u8>],
) -> Result<Vec<Transform>, GltfError> {
    let Some(value) = node.extension_value(EXT_MESH_GPU_INSTANCING) else {
        return Ok(Vec::new());
    };
    if node.mesh().is_none() {
        warn!(
            "Ignoring {EXT_MESH_GPU_INSTANCING} on node {}, which has no mesh",
            node.index()
        );
        return Ok(Vec::new());
    }
    let invalid = |message: String| GltfError::InvalidMeshGpuInstancing {
        node: node.index(),
        message,
    };
    let extension = NodeExtension::deserialize(value).map_err(|err| invalid(err.to_string()))?;

    let mut instance_count = None;
    let (mut translations, mut rotations, mut scales) = (None, None, None);
    for (name, &index) in &extension.attributes {
        let accessor = document
            .accessors()
            .nth(index)
            .ok_or_else(|| invalid(format!("{name} accessor {index} does not exist")))?;
        if !layout_is_valid(&accessor, buffer_data) {
            return Err(invalid(format!(
                "{name} has no buffer view or does not fit in its buffer views"
            )));
        }
        let count = *instance_count.get_or_insert(accessor.count());
        if count != accessor.count() {
            return Err(invalid(format!(
                "{name} has {} elements, other attributes have {count}",
                accessor.count()
            )));
        }
        match name.as_str() {
            "TRANSLATION" => translations = Some(accessor),
            "ROTATION" => rotations = Some(accessor),
            "SCALE" => scales = Some(accessor),
            _ => {}
        }
    }
    let count = instance_count.ok_or_else(|| invalid("attributes is empty".into()))?;

    let read_vec3 = |accessor: Accessor, name| match (accessor.dimensions(), accessor.data_type()) {
        (Dimensions::Vec3, DataType::F32) => {
            Ok(read::<[f32; 3]>(accessor, buffer_data, invalid, name)?.map(Vec3::from))
        }
        _ => Err(invalid(format!("{name} must be a float VEC3 accessor"))),
    };

    let mut transforms = vec![Transform::IDENTITY; count];
    if let Some(accessor) = translations {
        for (transform, translation) in transforms
            .iter_mut()
            .zip(read_vec3(accessor, "TRANSLATION")?)
        {
            transform.translation = translation;
        }
    }
    if let Some(accessor) = scales {
        for (transform, scale) in transforms.iter_mut().zip(read_vec3(accessor, "SCALE")?) {
            transform.scale = scale;
        }
    }
    if let Some(accessor) = rotations {
        let name = "ROTATION";
        let rotations = match (accessor.dimensions(), accessor.data_type()) {
            (Dimensions::Vec4, DataType::F32) => {
                Rotations::F32(read(accessor, buffer_data, invalid, name)?)
            }
            (Dimensions::Vec4, DataType::I8) if accessor.normalized() => {
                Rotations::I8(read(accessor, buffer_data, invalid, name)?)
            }
            (Dimensions::Vec4, DataType::I16) if accessor.normalized() => {
                Rotations::I16(read(accessor, buffer_data, invalid, name)?)
            }
            _ => {
                return Err(invalid(format!(
                    "{name} must be a float, normalized byte or normalized short VEC4 accessor"
                )));
            }
        };
        for (transform, rotation) in transforms.iter_mut().zip(rotations.into_f32()) {
            // Dequantized rotations are only approximately unit length.
            transform.rotation = Vec4::from(rotation)
                .try_normalize()
                .map_or(Quat::IDENTITY, Quat::from_vec4);
        }
    }

    Ok(transforms)
}

/// Iterates over the elements of `accessor`, whose type the caller checks.
fn read<'a, 's, T: Item>(
    accessor: Accessor<'a>,
    buffer_data: &'s [Vec<u8>],
    invalid: impl Fn(String) -> GltfError,
    name: &str,
) -> Result<Iter<'s, T>, GltfError> {
    Iter::new(accessor, |buffer: gltf::Buffer| {
        buffer_data.get(buffer.index()).map(Vec::as_slice)
    })
    .ok_or_else(|| invalid(format!("{name} has no readable data")))
}

/// Whether `accessor` has a buffer view and its elements, including sparse
/// indices and values, lie within their buffer views and the loaded buffer
/// data.
///
/// [`Iter`] doesn't check for an empty accessor, a stride shorter than an
/// element or arithmetic overflow, and custom attributes are never read
/// through it.
///
/// Requiring a buffer view limits the instance count to what the file's data
/// can hold.
fn layout_is_valid(accessor: &Accessor, buffer_data: &[Vec<u8>]) -> bool {
    let fits = |count: usize, size: usize, offset: usize, view: gltf::buffer::View| {
        let stride = view.stride().unwrap_or(size);
        let buffer_len = buffer_data.get(view.buffer().index()).map_or(0, Vec::len);
        stride >= size
            && view
                .offset()
                .checked_add(view.length())
                .is_some_and(|end| end <= buffer_len)
            && count
                .checked_sub(1)
                .and_then(|last| last.checked_mul(stride))
                .and_then(|end| end.checked_add(offset)?.checked_add(size))
                .is_some_and(|end| end <= view.length())
    };
    let size = accessor.size();
    accessor
        .view()
        .is_some_and(|view| fits(accessor.count(), size, accessor.offset(), view))
        && accessor.sparse().is_none_or(|sparse| {
            let (indices, values) = (sparse.indices(), sparse.values());
            fits(
                sparse.count(),
                indices.index_type().size(),
                indices.offset(),
                indices.view(),
            ) && fits(sparse.count(), size, values.offset(), values.view())
        })
}
