use fixedbitset::FixedBitSet;
use gltf::Gltf;

use crate::{ext::node::NodeExt, GltfError};

pub trait GltfExt {
    #[expect(
        clippy::result_large_err,
        reason = "need to be signature compatible with `load_gltf`"
    )]
    fn check_for_cycles(&self) -> Result<(), GltfError>;
}

impl GltfExt for Gltf {
    /// Checks all glTF nodes for cycles, starting at the scene root.
    fn check_for_cycles(&self) -> Result<(), GltfError> {
        // Initialize with the scene roots.
        let mut roots = FixedBitSet::with_capacity(self.nodes().len());
        for root in self.scenes().flat_map(|scene| scene.nodes()) {
            roots.insert(root.index());
        }

        // Check each one.
        let mut visited = FixedBitSet::with_capacity(self.nodes().len());
        for root in roots.ones() {
            let Some(node) = self.nodes().nth(root) else {
                unreachable!("Index of a root node should always exist.");
            };
            node.check_is_part_of_cycle(&mut visited)?;
        }

        Ok(())
    }
}
