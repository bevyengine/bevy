use alloc::collections::VecDeque;

use gltf::Node;

use bevy_pbr::MAX_JOINTS;
use bevy_utils::{tracing::warn, HashMap, HashSet};

use super::GltfError;

/// Iterator for a Gltf tree.
///
/// It resolves a Gltf tree and allows for a safe Gltf nodes iteration,
/// putting dependent nodes before dependencies.
pub struct GltfTreeIterator<'a> {
    nodes: Vec<Node<'a>>,
}

impl<'a> GltfTreeIterator<'a> {
    #[allow(clippy::result_large_err)]
    pub fn try_new(gltf: &'a gltf::Gltf) -> Result<Self, GltfError> {
        let nodes = gltf.nodes().collect::<Vec<_>>();

        let mut empty_children = VecDeque::new();
        let mut parents = vec![None; nodes.len()];
        let mut unprocessed_nodes = nodes
            .into_iter()
            .enumerate()
            .map(|(i, node)| {
                let children = node
                    .children()
                    .map(|child| child.index())
                    .collect::<HashSet<_>>();
                for &child in &children {
                    let parent = parents.get_mut(child).unwrap();
                    *parent = Some(i);
                }
                if children.is_empty() {
                    empty_children.push_back(i);
                }
                (i, (node, children))
            })
            .collect::<HashMap<_, _>>();

        let mut nodes = Vec::new();
        let mut warned_about_max_joints = HashSet::new();
        while let Some(index) = empty_children.pop_front() {
            if let Some(skin) = unprocessed_nodes.get(&index).unwrap().0.skin() {
                if skin.joints().len() > MAX_JOINTS && warned_about_max_joints.insert(skin.index())
                {
                    warn!(
                        "The glTF skin {:?} has {} joints, but the maximum supported is {}",
                        skin.name()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| skin.index().to_string()),
                        skin.joints().len(),
                        MAX_JOINTS
                    );
                }

                let skin_has_dependencies = skin
                    .joints()
                    .any(|joint| unprocessed_nodes.contains_key(&joint.index()));

                if skin_has_dependencies && unprocessed_nodes.len() != 1 {
                    empty_children.push_back(index);
                    continue;
                }
            }

            let (node, children) = unprocessed_nodes.remove(&index).unwrap();
            assert!(children.is_empty());
            nodes.push(node);

            if let Some(parent_index) = parents[index] {
                let (_, parent_children) = unprocessed_nodes.get_mut(&parent_index).unwrap();

                assert!(parent_children.remove(&index));
                if parent_children.is_empty() {
                    empty_children.push_back(parent_index);
                }
            }
        }

        if !unprocessed_nodes.is_empty() {
            return Err(GltfError::CircularChildren(format!(
                "{:?}",
                unprocessed_nodes
                    .iter()
                    .map(|(k, _v)| *k)
                    .collect::<Vec<_>>(),
            )));
        }

        nodes.reverse();
        Ok(Self {
            nodes: nodes.into_iter().collect(),
        })
    }
}

impl<'a> Iterator for GltfTreeIterator<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.nodes.pop()
    }
}

impl<'a> ExactSizeIterator for GltfTreeIterator<'a> {
    fn len(&self) -> usize {
        self.nodes.len()
    }
}
