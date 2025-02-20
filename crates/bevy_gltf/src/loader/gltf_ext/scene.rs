use bevy_ecs::name::Name;
use bevy_math::{Mat4, Vec3};
use bevy_transform::components::Transform;

use gltf::scene::Node;

use fixedbitset::FixedBitSet;
use itertools::Itertools;

#[cfg(feature = "bevy_animation")]
use bevy_platform_support::collections::{HashMap, HashSet};

use crate::GltfError;

pub trait NodeExt {
    fn node_name(&self) -> Name;

    fn node_transform(&self) -> Transform;

    #[expect(
        clippy::result_large_err,
        reason = "need to be signature compatible with `load_gltf`"
    )]
    fn check_is_part_of_cycle(&self, visited: &mut FixedBitSet) -> Result<(), GltfError>;

    #[cfg(feature = "bevy_animation")]
    fn collect_path(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    );
}

impl NodeExt for Node<'_> {
    fn node_name(&self) -> Name {
        let name = self
            .name()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("GltfNode{}", self.index()));
        Name::new(name)
    }

    /// Calculate the transform of gLTF node.
    ///
    /// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
    /// on [`Node::transform()`](gltf::Node::transform) directly because it uses optimized glam types and
    /// if `libm` feature of `bevy_math` crate is enabled also handles cross
    /// platform determinism properly.
    fn node_transform(&self) -> Transform {
        match self.transform() {
            gltf::scene::Transform::Matrix { matrix } => {
                Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
            }
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => Transform {
                translation: Vec3::from(translation),
                rotation: bevy_math::Quat::from_array(rotation),
                scale: Vec3::from(scale),
            },
        }
    }

    /// Check if [`Node`] is part of cycle
    fn check_is_part_of_cycle(&self, visited: &mut FixedBitSet) -> Result<(), GltfError> {
        // Do we have a cycle?
        if visited.contains(self.index()) {
            return Err(GltfError::CircularChildren(format!(
                "glTF nodes form a cycle: {} -> {}",
                visited.ones().map(|bit| bit.to_string()).join(" -> "),
                self.index()
            )));
        }

        // Recurse.
        visited.insert(self.index());
        for kid in self.children() {
            kid.check_is_part_of_cycle(visited)?;
        }
        visited.remove(self.index());

        Ok(())
    }

    #[cfg(feature = "bevy_animation")]
    fn collect_path(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    ) {
        let mut path = current_path.to_owned();
        path.push(self.node_name());
        visited.insert(self.index());
        for child in self.children() {
            if !visited.contains(&child.index()) {
                child.collect_path(&path, paths, root_index, visited);
            }
        }
        paths.insert(self.index(), (root_index, path));
    }
}
