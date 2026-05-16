use bevy_ecs::name::Name;
use bevy_math::{Mat4, Quat, Vec3};
use bevy_transform::components::Transform;

use gltf::{scene::Node, Gltf};

use fixedbitset::FixedBitSet;
use itertools::Itertools;

#[cfg(feature = "bevy_animation")]
use bevy_platform::collections::{HashMap, HashSet};

use crate::{
    convert_coordinates::{HierarchyConverter, ResolvedConvertCoordinates},
    GltfError,
};

pub(crate) fn node_name(node: &Node) -> Name {
    let name = node
        .name()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("GltfNode{}", node.index()));
    Name::new(name)
}

/// Calculate the transform of gLTF [`Node`].
///
/// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
/// on [`Node::transform()`](gltf::Node::transform) directly because it uses optimized glam types and
/// if `libm` feature of `bevy_math` crate is enabled also handles cross
/// platform determinism properly.
fn node_transform(node: &Node, conversion: &HierarchyConverter) -> Transform {
    let unconverted = match node.transform() {
        gltf::scene::Transform::Matrix { matrix } => {
            Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Transform {
            translation: Vec3::from(translation),
            rotation: Quat::from_array(rotation),
            scale: Vec3::from(scale),
        },
    };

    conversion.convert_transform(unconverted)
}

/// Returns the parent of each node, indexed by `Node::index`.
pub(crate) fn node_parents<'a>(gltf: &'a Gltf) -> Vec<Option<Node<'a>>> {
    let mut parent_indices = vec![Option::<usize>::None; gltf.nodes().len()];

    for node in gltf.nodes() {
        for child in node.children() {
            parent_indices[child.index()] = Some(node.index());
        }
    }

    parent_indices
        .into_iter()
        .map(|i| i.and_then(|i| gltf.nodes().nth(i)))
        .collect()
}

/// Returns the transform of each node, indexed by `Node::index`.
pub(crate) fn node_transforms<'a>(
    gltf: &'a Gltf,
    parents: &[Option<Node<'a>>],
    convert_coordinates: &ResolvedConvertCoordinates,
) -> Vec<Transform> {
    gltf.nodes()
        .map(|node| {
            node_transform(
                &node,
                &convert_coordinates.node_hierarchy_converter(&node, parents),
            )
        })
        .collect()
}

#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::result_large_err,
        reason = "need to be signature compatible with `load_gltf`"
    )
)]
/// Check if [`Node`] is part of cycle
pub(crate) fn check_is_part_of_cycle(
    node: &Node,
    visited: &mut FixedBitSet,
) -> Result<(), GltfError> {
    // Do we have a cycle?
    if visited.contains(node.index()) {
        return Err(GltfError::CircularChildren(format!(
            "glTF nodes form a cycle: {} -> {}",
            visited.ones().map(|bit| bit.to_string()).join(" -> "),
            node.index()
        )));
    }

    // Recurse.
    visited.insert(node.index());
    for kid in node.children() {
        check_is_part_of_cycle(&kid, visited)?;
    }
    visited.remove(node.index());

    Ok(())
}

#[cfg(feature = "bevy_animation")]
pub(crate) fn collect_path(
    node: &Node,
    current_path: &[Name],
    paths: &mut HashMap<usize, (usize, Vec<Name>)>,
    root_index: usize,
    visited: &mut HashSet<usize>,
) {
    let mut path = current_path.to_owned();
    path.push(node_name(node));
    visited.insert(node.index());
    for child in node.children() {
        if !visited.contains(&child.index()) {
            collect_path(&child, &path, paths, root_index, visited);
        }
    }
    paths.insert(node.index(), (root_index, path));
}
