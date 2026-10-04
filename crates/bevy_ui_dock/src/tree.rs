use alloc::{collections::BTreeMap, string::String, vec, vec::Vec};

use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui_widgets::ControlOrientation;
use thiserror::Error;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
#[cfg(feature = "serialize")]
use serde::{Deserialize, Serialize};

/// Identifies a node in a [`DockTree`]. Ids are never reused within a tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Clone, Debug, PartialEq, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct NodeId(u64);

/// Identifies a tab in a [`DockTree`]. A tab keeps its id when it moves between leaves, and
/// ids are never reused within a tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Clone, Debug, PartialEq, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct TabId(u64);

/// Names the content shown by a tab. The app maps each key to the content it builds for it.
///
/// Several tabs may share a key, for example two views of the same kind of panel.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[reflect(Clone, Debug, PartialEq, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct PanelKey(pub String);

impl PanelKey {
    /// Returns the key as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for PanelKey {
    fn from(value: &str) -> Self {
        Self(value.into())
    }
}

impl From<String> for PanelKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// An edge of a node, used to place a new leaf beside it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Reflect)]
#[reflect(Clone, Debug, PartialEq, Hash)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Edge {
    /// Above the node.
    Top,
    /// Below the node.
    Bottom,
    /// Left of the node.
    Left,
    /// Right of the node.
    Right,
}

impl Edge {
    /// The orientation of a split that places a node at this edge.
    pub fn orientation(self) -> ControlOrientation {
        match self {
            Edge::Top | Edge::Bottom => ControlOrientation::Vertical,
            Edge::Left | Edge::Right => ControlOrientation::Horizontal,
        }
    }

    /// Returns true for [`Edge::Top`] and [`Edge::Left`], which come first in a split.
    pub fn is_start(self) -> bool {
        matches!(self, Edge::Top | Edge::Left)
    }
}

/// A tab in a [`DockLeaf`].
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct DockTab {
    /// The id of this tab.
    pub id: TabId,
    /// The content this tab shows.
    pub panel: PanelKey,
}

/// A tab group: an ordered list of tabs, one of which is active.
#[derive(Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct DockLeaf {
    tabs: Vec<DockTab>,
    active: Option<TabId>,
    persistent: bool,
}

impl DockLeaf {
    /// The tabs in display order.
    pub fn tabs(&self) -> &[DockTab] {
        &self.tabs
    }

    /// The id of the active tab, or `None` if the leaf is empty.
    pub fn active(&self) -> Option<TabId> {
        self.active
    }

    /// The active tab, or `None` if the leaf is empty.
    pub fn active_tab(&self) -> Option<&DockTab> {
        self.active
            .and_then(|id| self.tabs.iter().find(|t| t.id == id))
    }

    /// The position of a tab in this leaf.
    pub fn tab_index(&self, tab: TabId) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == tab)
    }

    /// Whether this leaf is kept when its last tab is removed.
    pub fn is_persistent(&self) -> bool {
        self.persistent
    }
}

/// A child of a [`DockSplit`].
#[derive(Copy, Clone, Debug, PartialEq, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct SplitChild {
    /// The child node.
    pub node: NodeId,
    /// The flex weight of the child along the split axis, as in [`Pane::size`].
    ///
    /// [`Pane::size`]: bevy_ui_widgets::Pane::size
    pub size: f32,
}

/// Two or more nodes laid out side by side along an orientation.
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct DockSplit {
    orientation: ControlOrientation,
    children: Vec<SplitChild>,
}

impl DockSplit {
    /// The axis the children are laid out along, as in [`SplitPane::orientation`].
    ///
    /// [`SplitPane::orientation`]: bevy_ui_widgets::SplitPane::orientation
    pub fn orientation(&self) -> ControlOrientation {
        self.orientation
    }

    /// The children in layout order: left to right, or top to bottom.
    pub fn children(&self) -> &[SplitChild] {
        &self.children
    }
}

/// A node in a [`DockTree`].
#[derive(Clone, Debug, PartialEq, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum DockNode {
    /// A tab group.
    Leaf(DockLeaf),
    /// A split between child nodes.
    Split(DockSplit),
}

impl DockNode {
    /// Returns the leaf if this node is one.
    pub fn as_leaf(&self) -> Option<&DockLeaf> {
        match self {
            DockNode::Leaf(leaf) => Some(leaf),
            DockNode::Split(_) => None,
        }
    }

    /// Returns the split if this node is one.
    pub fn as_split(&self) -> Option<&DockSplit> {
        match self {
            DockNode::Split(split) => Some(split),
            DockNode::Leaf(_) => None,
        }
    }
}

/// An error from an operation on a [`DockTree`].
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockError {
    /// The node does not exist.
    #[error("dock node {0:?} does not exist")]
    NodeNotFound(NodeId),
    /// The node is a split where a leaf was expected.
    #[error("dock node {0:?} is not a leaf")]
    NotALeaf(NodeId),
    /// The node is a leaf where a split was expected.
    #[error("dock node {0:?} is not a split")]
    NotASplit(NodeId),
    /// The tab does not exist.
    #[error("dock tab {0:?} does not exist")]
    TabNotFound(TabId),
    /// The tab exists but is not in the given leaf.
    #[error("dock tab {tab:?} is not in leaf {leaf:?}")]
    TabNotInLeaf {
        /// The leaf that was searched.
        leaf: NodeId,
        /// The tab that was not found.
        tab: TabId,
    },
    /// Split sizes must be finite, positive, and one per child.
    #[error("split sizes must be finite, positive, and one per child")]
    InvalidSizes,
}

/// A dock layout: a tree of tab groups and the splits between them.
///
/// The tree always has a root node. A new tree's root is an empty leaf.
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
#[reflect(Component, Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(Serialize, Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct DockTree {
    nodes: BTreeMap<NodeId, DockNode>,
    root: NodeId,
    next_node: u64,
    next_tab: u64,
}

impl Default for DockTree {
    fn default() -> Self {
        let root = NodeId(0);
        Self {
            nodes: BTreeMap::from([(root, DockNode::Leaf(DockLeaf::default()))]),
            root,
            next_node: 1,
            next_tab: 0,
        }
    }
}

impl DockTree {
    /// Creates a tree whose root is an empty leaf.
    pub fn new() -> Self {
        Self::default()
    }

    /// The root node.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Returns a node by id.
    pub fn node(&self, id: NodeId) -> Option<&DockNode> {
        self.nodes.get(&id)
    }

    /// Returns a leaf by id, or `None` if it does not exist or is a split.
    pub fn leaf(&self, id: NodeId) -> Option<&DockLeaf> {
        self.node(id).and_then(DockNode::as_leaf)
    }

    /// Every node reachable from the root with its depth, in depth-first order.
    pub fn iter_dfs(&self) -> Vec<(NodeId, usize)> {
        let mut out = Vec::new();
        let mut stack = vec![(self.root, 0)];
        while let Some((id, depth)) = stack.pop() {
            out.push((id, depth));
            if let Some(DockNode::Split(split)) = self.nodes.get(&id) {
                stack.extend(split.children.iter().rev().map(|c| (c.node, depth + 1)));
            }
        }
        out
    }

    /// Every leaf reachable from the root, in depth-first order.
    pub fn leaves(&self) -> impl Iterator<Item = (NodeId, &DockLeaf)> {
        self.iter_dfs()
            .into_iter()
            .filter_map(|(id, _)| self.leaf(id).map(|leaf| (id, leaf)))
    }

    /// Every tab with the leaf that holds it, in depth-first then display order.
    pub fn tabs(&self) -> impl Iterator<Item = (NodeId, &DockTab)> {
        self.leaves()
            .flat_map(|(id, leaf)| leaf.tabs.iter().map(move |tab| (id, tab)))
    }

    /// The leaf that holds a tab.
    pub fn find_leaf_for_tab(&self, tab: TabId) -> Option<NodeId> {
        self.nodes.iter().find_map(|(id, node)| match node {
            DockNode::Leaf(leaf) if leaf.tab_index(tab).is_some() => Some(*id),
            _ => None,
        })
    }

    /// The first tab showing a panel, with the leaf that holds it.
    pub fn find_panel(&self, panel: &str) -> Option<(NodeId, TabId)> {
        self.tabs()
            .find(|(_, tab)| tab.panel.as_str() == panel)
            .map(|(leaf, tab)| (leaf, tab.id))
    }

    /// The split that contains a node, or `None` for the root or an unknown node.
    pub fn parent_of(&self, child: NodeId) -> Option<NodeId> {
        self.nodes.iter().find_map(|(id, node)| match node {
            DockNode::Split(split) if split.children.iter().any(|c| c.node == child) => Some(*id),
            _ => None,
        })
    }

    /// Appends a tab showing `panel` to a leaf and makes it active.
    pub fn add_tab(
        &mut self,
        leaf: NodeId,
        panel: impl Into<PanelKey>,
    ) -> Result<TabId, DockError> {
        self.leaf_mut(leaf)?;
        let tab = self.fresh_tab(panel.into());
        let id = tab.id;
        let target = self.leaf_mut(leaf)?;
        target.tabs.push(tab);
        target.active = Some(id);
        Ok(id)
    }

    /// Places a new leaf holding a tab for `panel` at an edge of `target`, which may be a leaf
    /// or a split. The new leaf takes half of the space `target` had.
    ///
    /// Returns the new leaf and tab.
    pub fn split(
        &mut self,
        target: NodeId,
        edge: Edge,
        panel: impl Into<PanelKey>,
    ) -> Result<(NodeId, TabId), DockError> {
        self.node_exists(target)?;
        let tab = self.fresh_tab(panel.into());
        let id = tab.id;
        let leaf = self.split_with(target, edge, tab);
        Ok((leaf, id))
    }

    /// Moves a tab into a leaf at `index`, or to the end if `index` is `None`, and makes it
    /// active. The index is clamped and counts positions after the tab is taken out, so this
    /// also reorders tabs within a leaf.
    pub fn move_tab(
        &mut self,
        tab: TabId,
        to: NodeId,
        index: Option<usize>,
    ) -> Result<(), DockError> {
        self.leaf_mut(to)?;
        let (_, entry) = self.take_tab(tab)?;
        let target = self.leaf_mut(to)?;
        let index = index.unwrap_or(target.tabs.len()).min(target.tabs.len());
        target.tabs.insert(index, entry);
        target.active = Some(tab);
        self.simplify();
        Ok(())
    }

    /// Moves a tab into a new leaf at an edge of `target`, as in [`DockTree::split`].
    ///
    /// Returns the new leaf.
    pub fn move_tab_to_edge(
        &mut self,
        tab: TabId,
        target: NodeId,
        edge: Edge,
    ) -> Result<NodeId, DockError> {
        self.node_exists(target)?;
        let (_, entry) = self.take_tab(tab)?;
        Ok(self.split_with(target, edge, entry))
    }

    /// Removes a tab and returns it. A leaf left empty is removed unless it is the root or
    /// persistent.
    pub fn remove_tab(&mut self, tab: TabId) -> Result<DockTab, DockError> {
        let (_, entry) = self.take_tab(tab)?;
        self.simplify();
        Ok(entry)
    }

    /// Makes a tab in a leaf active.
    pub fn set_active(&mut self, leaf: NodeId, tab: TabId) -> Result<(), DockError> {
        let target = self.leaf_mut(leaf)?;
        if target.tab_index(tab).is_none() {
            return Err(DockError::TabNotInLeaf { leaf, tab });
        }
        target.active = Some(tab);
        Ok(())
    }

    /// Sets whether a leaf is kept when its last tab is removed.
    pub fn set_persistent(&mut self, leaf: NodeId, persistent: bool) -> Result<(), DockError> {
        self.leaf_mut(leaf)?.persistent = persistent;
        Ok(())
    }

    /// Sets the sizes of a split's children, one per child in layout order.
    ///
    /// This takes the same values a split pane reports in its `ValueChange<Vec<f32>>`.
    pub fn set_sizes(&mut self, split: NodeId, sizes: &[f32]) -> Result<(), DockError> {
        let DockNode::Split(target) = self
            .nodes
            .get_mut(&split)
            .ok_or(DockError::NodeNotFound(split))?
        else {
            return Err(DockError::NotASplit(split));
        };
        if sizes.len() != target.children.len() || sizes.iter().any(|s| !s.is_finite() || *s <= 0.0)
        {
            return Err(DockError::InvalidSizes);
        }
        for (child, size) in target.children.iter_mut().zip(sizes) {
            child.size = *size;
        }
        Ok(())
    }

    /// Normalizes the tree. Empty leaves are removed unless they are the root or persistent,
    /// splits left with one child are replaced by that child, and a split nested directly in
    /// a split of the same orientation is merged into it, keeping the space each node had.
    ///
    /// Operations that remove or add nodes call this themselves.
    pub fn simplify(&mut self) {
        let root = self.root;
        self.root = match self.simplify_node(root, root) {
            Some(id) => id,
            None => {
                let id = self.fresh_node();
                self.nodes.insert(id, DockNode::Leaf(DockLeaf::default()));
                id
            }
        };
    }

    fn simplify_node(&mut self, id: NodeId, root: NodeId) -> Option<NodeId> {
        let (orientation, old_children) = match self.nodes.get(&id)? {
            DockNode::Leaf(leaf) => {
                if leaf.tabs.is_empty() && !leaf.persistent && id != root {
                    self.nodes.remove(&id);
                    return None;
                }
                return Some(id);
            }
            DockNode::Split(split) => (split.orientation, split.children.clone()),
        };
        let mut children = Vec::with_capacity(old_children.len());
        for child in old_children {
            let Some(node) = self.simplify_node(child.node, root) else {
                continue;
            };
            match self.nodes.get(&node) {
                Some(DockNode::Split(inner)) if inner.orientation == orientation => {
                    let total: f32 = inner.children.iter().map(|c| c.size).sum();
                    children.extend(inner.children.iter().map(|c| SplitChild {
                        node: c.node,
                        size: child.size * c.size / total,
                    }));
                    self.nodes.remove(&node);
                }
                _ => children.push(SplitChild {
                    node,
                    size: child.size,
                }),
            }
        }
        match children.len() {
            0 => {
                self.nodes.remove(&id);
                None
            }
            1 => {
                self.nodes.remove(&id);
                Some(children[0].node)
            }
            _ => {
                if let Some(DockNode::Split(split)) = self.nodes.get_mut(&id) {
                    split.children = children;
                }
                Some(id)
            }
        }
    }

    fn split_with(&mut self, target: NodeId, edge: Edge, tab: DockTab) -> NodeId {
        let parent = self.parent_of(target);
        let leaf = self.fresh_node();
        self.nodes.insert(
            leaf,
            DockNode::Leaf(DockLeaf {
                active: Some(tab.id),
                tabs: vec![tab],
                persistent: false,
            }),
        );
        let pair = if edge.is_start() {
            [leaf, target]
        } else {
            [target, leaf]
        };
        let split = self.fresh_node();
        self.nodes.insert(
            split,
            DockNode::Split(DockSplit {
                orientation: edge.orientation(),
                children: pair.map(|node| SplitChild { node, size: 1.0 }).into(),
            }),
        );
        match parent.and_then(|p| self.nodes.get_mut(&p)) {
            Some(DockNode::Split(parent)) => {
                for child in &mut parent.children {
                    if child.node == target {
                        child.node = split;
                    }
                }
            }
            _ => self.root = split,
        }
        self.simplify();
        leaf
    }

    fn take_tab(&mut self, tab: TabId) -> Result<(NodeId, DockTab), DockError> {
        let leaf = self
            .find_leaf_for_tab(tab)
            .ok_or(DockError::TabNotFound(tab))?;
        let source = self.leaf_mut(leaf)?;
        let index = source.tab_index(tab).ok_or(DockError::TabNotFound(tab))?;
        let entry = source.tabs.remove(index);
        if source.active == Some(tab) {
            let next = index.min(source.tabs.len().saturating_sub(1));
            source.active = source.tabs.get(next).map(|t| t.id);
        }
        Ok((leaf, entry))
    }

    fn leaf_mut(&mut self, id: NodeId) -> Result<&mut DockLeaf, DockError> {
        match self.nodes.get_mut(&id) {
            Some(DockNode::Leaf(leaf)) => Ok(leaf),
            Some(DockNode::Split(_)) => Err(DockError::NotALeaf(id)),
            None => Err(DockError::NodeNotFound(id)),
        }
    }

    fn node_exists(&self, id: NodeId) -> Result<(), DockError> {
        if self.nodes.contains_key(&id) {
            Ok(())
        } else {
            Err(DockError::NodeNotFound(id))
        }
    }

    fn fresh_node(&mut self) -> NodeId {
        let id = NodeId(self.next_node);
        self.next_node += 1;
        id
    }

    fn fresh_tab(&mut self, panel: PanelKey) -> DockTab {
        let id = TabId(self.next_tab);
        self.next_tab += 1;
        DockTab { id, panel }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{string::ToString, vec::Vec};

    fn tree_with(panels: &[&str]) -> (DockTree, NodeId) {
        let mut tree = DockTree::new();
        let root = tree.root();
        for panel in panels {
            tree.add_tab(root, *panel).unwrap();
        }
        (tree, root)
    }

    fn panels(tree: &DockTree, leaf: NodeId) -> Vec<String> {
        tree.leaf(leaf)
            .unwrap()
            .tabs()
            .iter()
            .map(|t| t.panel.as_str().to_string())
            .collect()
    }

    fn active_panel(tree: &DockTree, leaf: NodeId) -> Option<&str> {
        tree.leaf(leaf)?.active_tab().map(|t| t.panel.as_str())
    }

    fn tab(tree: &DockTree, panel: &str) -> TabId {
        tree.find_panel(panel).unwrap().1
    }

    fn root_split(tree: &DockTree) -> &DockSplit {
        tree.node(tree.root()).unwrap().as_split().unwrap()
    }

    fn child_nodes(split: &DockSplit) -> Vec<NodeId> {
        split.children().iter().map(|c| c.node).collect()
    }

    #[test]
    fn new_tree_has_empty_root_leaf() {
        let tree = DockTree::new();
        assert_eq!(tree.leaves().count(), 1);
        assert!(tree.leaf(tree.root()).unwrap().tabs().is_empty());
    }

    #[test]
    fn added_tabs_get_unique_ids_and_become_active() {
        let (tree, root) = tree_with(&["a", "b"]);
        let leaf = tree.leaf(root).unwrap();
        assert_ne!(leaf.tabs()[0].id, leaf.tabs()[1].id);
        assert_eq!(leaf.active(), Some(leaf.tabs()[1].id));
    }

    #[test]
    fn split_right_wraps_target() {
        let (mut tree, root) = tree_with(&["a"]);
        let (new_leaf, _) = tree.split(root, Edge::Right, "b").unwrap();
        let split = root_split(&tree);
        assert_eq!(split.orientation(), ControlOrientation::Horizontal);
        assert_eq!(child_nodes(split), vec![root, new_leaf]);
        assert!(split.children().iter().all(|c| c.size == 1.0));
        assert_eq!(panels(&tree, root), vec!["a"]);
        assert_eq!(panels(&tree, new_leaf), vec!["b"]);
    }

    #[test]
    fn split_at_each_edge() {
        for (edge, orientation, new_first) in [
            (Edge::Top, ControlOrientation::Vertical, true),
            (Edge::Bottom, ControlOrientation::Vertical, false),
            (Edge::Left, ControlOrientation::Horizontal, true),
            (Edge::Right, ControlOrientation::Horizontal, false),
        ] {
            let (mut tree, root) = tree_with(&["a"]);
            let (new_leaf, _) = tree.split(root, edge, "b").unwrap();
            let split = root_split(&tree);
            assert_eq!(split.orientation(), orientation);
            let expected = if new_first {
                vec![new_leaf, root]
            } else {
                vec![root, new_leaf]
            };
            assert_eq!(child_nodes(split), expected);
        }
    }

    #[test]
    fn split_top_puts_new_first() {
        let (mut tree, root) = tree_with(&["a"]);
        let (new_leaf, _) = tree.split(root, Edge::Top, "b").unwrap();
        assert_eq!(child_nodes(root_split(&tree)), vec![new_leaf, root]);
    }

    #[test]
    fn split_bottom_puts_new_last() {
        let (mut tree, root) = tree_with(&["a"]);
        let (new_leaf, _) = tree.split(root, Edge::Bottom, "b").unwrap();
        assert_eq!(child_nodes(root_split(&tree)), vec![root, new_leaf]);
    }

    #[test]
    fn split_of_nested_leaf_preserves_other_sibling() {
        let (mut tree, root) = tree_with(&["a"]);
        let (right, _) = tree.split(root, Edge::Right, "b").unwrap();
        let (bottom, _) = tree.split(right, Edge::Bottom, "c").unwrap();

        assert_eq!(panels(&tree, root), vec!["a"]);
        assert_eq!(panels(&tree, right), vec!["b"]);
        assert_eq!(panels(&tree, bottom), vec!["c"]);
        let inner = tree.parent_of(right).unwrap();
        assert_eq!(tree.parent_of(inner), Some(tree.root()));
    }

    #[test]
    fn split_in_same_orientation_joins_parent() {
        let (mut tree, root) = tree_with(&["a"]);
        let (right, _) = tree.split(root, Edge::Right, "b").unwrap();
        let (middle, _) = tree.split(root, Edge::Right, "c").unwrap();

        let split = root_split(&tree);
        assert_eq!(child_nodes(split), vec![root, middle, right]);
        let sizes: Vec<f32> = split.children().iter().map(|c| c.size).collect();
        assert_eq!(sizes, vec![0.5, 0.5, 1.0]);
    }

    #[test]
    fn move_tab_relocates_and_activates() {
        let (mut tree, root) = tree_with(&["a", "b"]);
        let (right, _) = tree.split(root, Edge::Right, "c").unwrap();
        tree.move_tab(tab(&tree, "a"), right, None).unwrap();

        assert_eq!(panels(&tree, root), vec!["b"]);
        assert_eq!(panels(&tree, right), vec!["c", "a"]);
        assert_eq!(active_panel(&tree, right), Some("a"));
    }

    #[test]
    fn move_tab_within_same_leaf() {
        let (mut tree, root) = tree_with(&["a", "b", "c"]);
        let a = tab(&tree, "a");
        tree.move_tab(a, root, Some(2)).unwrap();
        assert_eq!(panels(&tree, root), vec!["b", "c", "a"]);
        tree.move_tab(a, root, Some(0)).unwrap();
        assert_eq!(panels(&tree, root), vec!["a", "b", "c"]);
        assert_eq!(tab(&tree, "a"), a);
    }

    #[test]
    fn move_last_tab_simplifies_tree() {
        let (mut tree, root) = tree_with(&["a"]);
        let (right, _) = tree.split(root, Edge::Right, "b").unwrap();
        tree.move_tab(tab(&tree, "a"), right, None).unwrap();

        assert_eq!(tree.root(), right);
        assert_eq!(tree.leaves().count(), 1);
        assert_eq!(panels(&tree, right), vec!["b", "a"]);
    }

    #[test]
    fn move_tab_to_edge_keeps_tab_id() {
        let (mut tree, root) = tree_with(&["a", "b"]);
        let b = tab(&tree, "b");
        let leaf = tree.move_tab_to_edge(b, root, Edge::Left).unwrap();

        assert_eq!(child_nodes(root_split(&tree)), vec![leaf, root]);
        assert_eq!(tree.find_panel("b"), Some((leaf, b)));
        assert_eq!(panels(&tree, root), vec!["a"]);
    }

    #[test]
    fn remove_last_tab_keeps_root_empty_leaf() {
        let (mut tree, root) = tree_with(&["a"]);
        let removed = tree.remove_tab(tab(&tree, "a")).unwrap();

        assert_eq!(removed.panel.as_str(), "a");
        assert_eq!(tree.root(), root);
        assert!(tree.leaf(root).unwrap().tabs().is_empty());
    }

    #[test]
    fn remove_last_tab_collapses_leaf_and_parent() {
        let (mut tree, root) = tree_with(&["a"]);
        let (right, _) = tree.split(root, Edge::Right, "b").unwrap();
        tree.split(right, Edge::Bottom, "c").unwrap();
        tree.set_sizes(tree.root(), &[1.0, 3.0]).unwrap();

        tree.remove_tab(tab(&tree, "c")).unwrap();

        let split = root_split(&tree);
        assert_eq!(child_nodes(split), vec![root, right]);
        assert_eq!(split.children()[1].size, 3.0);
        assert_eq!(tree.iter_dfs().len(), 3);
    }

    #[test]
    fn removing_active_tab_activates_neighbour() {
        let (mut tree, root) = tree_with(&["a", "b", "c"]);
        tree.set_active(root, tab(&tree, "b")).unwrap();
        tree.remove_tab(tab(&tree, "b")).unwrap();
        assert_eq!(active_panel(&tree, root), Some("c"));
        tree.remove_tab(tab(&tree, "c")).unwrap();
        assert_eq!(active_panel(&tree, root), Some("a"));
    }

    #[test]
    fn set_sizes_rejects_invalid_values() {
        let (mut tree, root) = tree_with(&["a"]);
        tree.split(root, Edge::Right, "b").unwrap();
        let split = tree.root();

        tree.set_sizes(split, &[2.0, 1.0]).unwrap();
        assert_eq!(root_split(&tree).children()[0].size, 2.0);
        for sizes in [&[1.0][..], &[0.0, 1.0], &[-1.0, 1.0], &[f32::NAN, 1.0]] {
            assert_eq!(tree.set_sizes(split, sizes), Err(DockError::InvalidSizes));
        }
        assert_eq!(
            tree.set_sizes(root, &[1.0]),
            Err(DockError::NotASplit(root))
        );
    }

    #[test]
    fn set_active_requires_tab_in_leaf() {
        let (mut tree, root) = tree_with(&["a", "b"]);
        let a = tab(&tree, "a");
        tree.set_active(root, a).unwrap();
        assert_eq!(active_panel(&tree, root), Some("a"));

        let stranger = TabId(9999);
        assert_eq!(
            tree.set_active(root, stranger),
            Err(DockError::TabNotInLeaf {
                leaf: root,
                tab: stranger
            })
        );
        assert_eq!(active_panel(&tree, root), Some("a"));
    }

    #[test]
    fn duplicate_panel_supported() {
        let (mut tree, root) = tree_with(&[]);
        let first = tree.add_tab(root, "outliner").unwrap();
        let second = tree.add_tab(root, "outliner").unwrap();
        assert_ne!(first, second);
        assert_eq!(panels(&tree, root), vec!["outliner", "outliner"]);

        tree.remove_tab(second).unwrap();
        let leaf = tree.leaf(root).unwrap();
        assert_eq!(leaf.tabs().len(), 1);
        assert_eq!(leaf.tabs()[0].id, first);
        assert_eq!(leaf.active(), Some(first));
    }

    #[test]
    fn persistent_leaf_kept_when_emptied() {
        let (mut tree, root) = tree_with(&["a"]);
        let (other, _) = tree.split(root, Edge::Right, "b").unwrap();
        tree.set_persistent(root, true).unwrap();
        tree.move_tab(tab(&tree, "a"), other, None).unwrap();

        let leaf = tree.leaf(root).unwrap();
        assert!(leaf.tabs().is_empty());
        assert!(leaf.is_persistent());
        assert_eq!(child_nodes(root_split(&tree)), vec![root, other]);
    }

    #[test]
    fn nested_split_chain_simplifies_when_drained() {
        let (mut tree, root) = tree_with(&["a"]);
        let (right, _) = tree.split(root, Edge::Right, "b").unwrap();
        tree.split(right, Edge::Bottom, "c").unwrap();

        tree.move_tab(tab(&tree, "b"), root, None).unwrap();
        tree.move_tab(tab(&tree, "c"), root, None).unwrap();

        assert_eq!(tree.root(), root);
        assert_eq!(tree.iter_dfs(), vec![(root, 0)]);
        assert_eq!(panels(&tree, root), vec!["a", "b", "c"]);
    }

    #[test]
    fn invalid_ids_return_errors() {
        let (mut tree, root) = tree_with(&["a"]);
        tree.split(root, Edge::Right, "b").unwrap();
        let split = tree.root();
        let missing_node = NodeId(9999);
        let missing_tab = TabId(9999);
        let before = tree.clone();

        assert_eq!(
            tree.add_tab(missing_node, "x"),
            Err(DockError::NodeNotFound(missing_node))
        );
        assert_eq!(tree.add_tab(split, "x"), Err(DockError::NotALeaf(split)));
        assert_eq!(
            tree.split(missing_node, Edge::Top, "x"),
            Err(DockError::NodeNotFound(missing_node))
        );
        assert_eq!(
            tree.move_tab(missing_tab, root, None),
            Err(DockError::TabNotFound(missing_tab))
        );
        assert_eq!(
            tree.move_tab(tab(&tree, "a"), split, None),
            Err(DockError::NotALeaf(split))
        );
        assert_eq!(
            tree.move_tab_to_edge(tab(&tree, "a"), missing_node, Edge::Top),
            Err(DockError::NodeNotFound(missing_node))
        );
        assert_eq!(
            tree.remove_tab(missing_tab),
            Err(DockError::TabNotFound(missing_tab))
        );
        assert_eq!(
            tree.set_persistent(missing_node, true),
            Err(DockError::NodeNotFound(missing_node))
        );
        assert_eq!(tree, before);
        assert_eq!(tree.parent_of(missing_node), None);
        assert_eq!(tree.find_leaf_for_tab(missing_tab), None);
    }

    #[cfg(feature = "serialize")]
    #[test]
    fn serde_round_trip() {
        let (mut tree, root) = tree_with(&["a"]);
        tree.split(root, Edge::Right, "b").unwrap();
        tree.set_sizes(tree.root(), &[2.0, 1.0]).unwrap();

        let text = ron::to_string(&tree).unwrap();
        let mut restored: DockTree = ron::from_str(&text).unwrap();
        assert_eq!(restored, tree);

        let c = restored.add_tab(root, "c").unwrap();
        assert!(tree.tabs().all(|(_, t)| t.id != c));
    }
}
