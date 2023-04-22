use bevy_ecs::prelude::Entity;
use bevy_ecs::system::Resource;
use bevy_utils::HashMap;
use slotmap::SlotMap;
use slotmap::SparseSecondaryMap;
use taffy::node::MeasureFunc;
use taffy::prelude::Node;
use taffy::tree::LayoutTree;

#[derive(Resource, Default)]
pub struct UiSurface {
    pub entity_to_taffy: HashMap<Entity, taffy::node::Node>,
    
    pub window_nodes: HashMap<Entity, taffy::node::Node>,
    
    pub nodes: SlotMap<Node, UiNodeData>,

    /// Functions/closures that compute the intrinsic size of leaf nodes
    pub measure_funcs: SparseSecondaryMap<Node, taffy::node::MeasureFunc>,

    /// The children of each node
    ///
    /// The indexes in the outer vector correspond to the position of the parent [`NodeData`]
    pub children: SlotMap<Node, Vec<Node>>,

    /// The parents of each node
    ///
    /// The indexes in the outer vector correspond to the position of the child [`NodeData`]
    pub parents: SlotMap<Node, Option<Node>>,
}

impl LayoutTree for UiSurface {
    type ChildIter<'a> =  core::slice::Iter<'a, taffy::prelude::Node>
    where
        Self: 'a;

        fn children(&self, node: Node) -> Self::ChildIter<'_> {
            self.children[node].iter()
        }
    
        fn child_count(&self, node: Node) -> usize {
            self.children[node].len()
        }
    
        fn is_childless(&self, node: Node) -> bool {
            self.children[node].is_empty()
        }
    
        fn parent(&self, node: Node) -> Option<Node> {
            self.parents.get(node).copied().flatten()
        }
    
        fn style(&self, node: Node) -> &taffy::style::Style {
            &self.nodes[node].style
        }
    
        fn layout(&self, node: Node) -> &taffy::prelude::Layout {
            &self.nodes[node].layout
        }
    
        fn layout_mut(&mut self, node: Node) -> &mut taffy::prelude::Layout {
            &mut self.nodes[node].layout
        }
    
        #[inline(always)]
        fn mark_dirty(&mut self, node: Node) -> taffy::error::TaffyResult<()> {
            self.mark_dirty_internal(node)
        }
    
        fn measure_node(
            &self,
            node: Node,
            known_dimensions: taffy::prelude::Size<Option<f32>>,
            available_space: taffy::prelude::Size<taffy::style::AvailableSpace>,
        ) -> taffy::prelude::Size<f32> {
            match &self.measure_funcs[node] {
                taffy::node::MeasureFunc::Raw(measure) => measure(known_dimensions, available_space),
    
                taffy::node::MeasureFunc::Boxed(measure) => (measure as &dyn Fn(_, _) -> _)(known_dimensions, available_space),
            }
        }
    
        fn needs_measure(&self, node: Node) -> bool {
            self.nodes[node].needs_measure && self.measure_funcs.get(node).is_some()
        }
    
        fn cache_mut(&mut self, node: Node, index: usize) -> &mut Option<taffy::layout::Cache> {
            &mut self.nodes[node].size_cache[index]
        }
    
        fn child(&self, node: Node, id: usize) -> Node {
            self.children[node][id]
        }
}


impl UiSurface {
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: SlotMap::with_capacity(capacity),
            children: SlotMap::with_capacity(capacity),
            parents: SlotMap::with_capacity(capacity),
            measure_funcs: SparseSecondaryMap::with_capacity(capacity),
            entity_to_taffy: HashMap::default(),
            window_nodes: HashMap::default(),
        }
    }

    /// Creates and adds a new unattached leaf node to the tree, and returns the [`Node`] of the new node
    pub fn new_leaf(&mut self, layout: taffy::style::Style) -> taffy::error::TaffyResult<Node> {
        let id = self.nodes.insert(UiNodeData::new(layout));
        let _ = self.children.insert(Vec::with_capacity(0));
        let _ = self.parents.insert(None);

        Ok(id)
    }

    /// Creates and adds a new unattached leaf node to the tree, and returns the [`Node`] of the new node
    ///
    /// Creates and adds a new leaf node with a supplied [`MeasureFunc`]
    pub fn new_leaf_with_measure(&mut self, layout: taffy::style::Style, measure: MeasureFunc) -> taffy::error::TaffyResult<Node> {
        let mut data = UiNodeData::new(layout);
        data.needs_measure = true;

        let id = self.nodes.insert(data);
        self.measure_funcs.insert(id, measure);

        let _ = self.children.insert(Vec::with_capacity(0));
        let _ = self.parents.insert(None);

        Ok(id)
    }

    /// Creates and adds a new node, which may have any number of `children`
    pub fn new_with_children(&mut self, layout: taffy::style::Style, children: &[Node]) -> taffy::error::TaffyResult<Node> {
        let id = self.nodes.insert(UiNodeData::new(layout));

        for child in children {
            self.parents[*child] = Some(id);
        }

        let _ = self.children.insert(children.iter().copied().collect::<_>());
        let _ = self.parents.insert(None);

        Ok(id)
    }

    /// Drops all nodes in the tree
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.children.clear();
        self.parents.clear();
    }

    /// Remove a specific [`Node`] from the tree and drops it
    ///
    /// Returns the id of the node removed.
    pub fn remove(&mut self, node: Node) -> taffy::error::TaffyResult<Node> {
        if let Some(parent) = self.parents[node] {
            if let Some(children) = self.children.get_mut(parent) {
                children.retain(|f| *f != node);
            }
        }

        let _ = self.children.remove(node);
        let _ = self.parents.remove(node);
        let _ = self.nodes.remove(node);

        Ok(node)
    }

    /// Sets the [`MeasureFunc`] of the associated node
    pub fn set_measure(&mut self, node: Node, measure: Option<MeasureFunc>) -> taffy::error::TaffyResult<()> {
        if let Some(measure) = measure {
            self.nodes[node].needs_measure = true;
            self.measure_funcs.insert(node, measure);
        } else {
            self.nodes[node].needs_measure = false;
            self.measure_funcs.remove(node);
        }

        self.mark_dirty_internal(node)?;

        Ok(())
    }

    /// Adds a `child` [`Node`] under the supplied `parent`
    pub fn add_child(&mut self, parent: Node, child: Node) -> taffy::error::TaffyResult<()> {
        self.parents[child] = Some(parent);
        self.children[parent].push(child);
        self.mark_dirty_internal(parent)?;

        Ok(())
    }

    /// Directly sets the `children` of the supplied `parent`
    pub fn set_children(&mut self, parent: Node, children: &[Node]) -> taffy::error::TaffyResult<()> {
        // Remove node as parent from all its current children.
        for child in &self.children[parent] {
            self.parents[*child] = None;
        }

        // Build up relation node <-> child
        for child in children {
            self.parents[*child] = Some(parent);
        }

        self.children[parent] = children.iter().copied().collect::<_>();

        self.mark_dirty_internal(parent)?;

        Ok(())
    }

    /// Removes the `child` of the parent `node`
    ///
    /// The child is not removed from the tree entirely, it is simply no longer attached to its previous parent.
    pub fn remove_child(&mut self, parent: Node, child: Node) -> taffy::error::TaffyResult<Node> {
        let index = self.children[parent].iter().position(|n| *n == child).unwrap();
        self.remove_child_at_index(parent, index)
    }

    /// Removes the child at the given `index` from the `parent`
    ///
    /// The child is not removed from the tree entirely, it is simply no longer attached to its previous parent.
    pub fn remove_child_at_index(&mut self, parent: Node, child_index: usize) -> taffy::error::TaffyResult<Node> {
        let child_count = self.children[parent].len();
        if child_index >= child_count {
            return Err(taffy::error::TaffyError::ChildIndexOutOfBounds { parent, child_index, child_count });
        }

        let child = self.children[parent].remove(child_index);
        self.parents[child] = None;

        self.mark_dirty_internal(parent)?;

        Ok(child)
    }

    /// Replaces the child at the given `child_index` from the `parent` node with the new `child` node
    ///
    /// The child is not removed from the tree entirely, it is simply no longer attached to its previous parent.
    pub fn replace_child_at_index(&mut self, parent: Node, child_index: usize, new_child: Node) -> taffy::error::TaffyResult<Node> {
        let child_count = self.children[parent].len();
        if child_index >= child_count {
            return Err(taffy::error::TaffyError::ChildIndexOutOfBounds { parent, child_index, child_count });
        }

        self.parents[new_child] = Some(parent);
        let old_child = core::mem::replace(&mut self.children[parent][child_index], new_child);
        self.parents[old_child] = None;

        self.mark_dirty_internal(parent)?;

        Ok(old_child)
    }

    /// Returns the child [`Node`] of the parent `node` at the provided `child_index`
    pub fn child_at_index(&self, parent: Node, child_index: usize) -> taffy::error::TaffyResult<Node> {
        let child_count = self.children[parent].len();
        if child_index >= child_count {
            return Err(taffy::error::TaffyError::ChildIndexOutOfBounds { parent, child_index, child_count });
        }

        Ok(self.children[parent][child_index])
    }

    /// Returns the number of children of the `parent` [`Node`]
    pub fn child_count(&self, parent: Node) -> taffy::error::TaffyResult<usize> {
        Ok(self.children[parent].len())
    }

    /// Returns a list of children that belong to the parent [`Node`]
    pub fn children(&self, parent: Node) -> taffy::error::TaffyResult<Vec<Node>> {
        Ok(self.children[parent].iter().copied().collect::<_>())
    }

    /// Sets the [`Style`] of the provided `node`
    pub fn set_style(&mut self, node: Node, style: taffy::style::Style) -> taffy::error::TaffyResult<()> {
        self.nodes[node].style = style;
        self.mark_dirty_internal(node)?;
        Ok(())
    }

    /// Gets the [`Style`] of the provided `node`
    pub fn style(&self, node: Node) -> taffy::error::TaffyResult<&taffy::style::Style> {
        Ok(&self.nodes[node].style)
    }

    /// Return this node layout relative to its parent
    pub fn layout(&self, node: Node) -> taffy::error::TaffyResult<&taffy::prelude::Layout> {
        Ok(&self.nodes[node].layout)
    }

    /// Marks the layout computation of this node and its children as outdated
    ///
    /// Performs a recursive depth-first search up the tree until the root node is reached
    ///
    /// WARNING: this will stack-overflow if the tree contains a cycle
    fn mark_dirty_internal(&mut self, node: Node) -> taffy::error::TaffyResult<()> {
        /// WARNING: this will stack-overflow if the tree contains a cycle
        fn mark_dirty_recursive(
            nodes: &mut SlotMap<Node, UiNodeData>,
            parents: &SlotMap<Node, Option<Node>>,
            node_id: Node,
        ) {
            nodes[node_id].mark_dirty();

            if let Some(Some(node)) = parents.get(node_id) {
                mark_dirty_recursive(nodes, parents, *node);
            }
        }

        mark_dirty_recursive(&mut self.nodes, &self.parents, node);

        Ok(())
    }

    /// Indicates whether the layout of this node (and its children) need to be recomputed
    pub fn dirty(&self, node: Node) -> taffy::error::TaffyResult<bool> {
        Ok(self.nodes[node].size_cache.iter().all(|entry| entry.is_none()))
    }

    /// Updates the stored layout of the provided `node` and its children
    pub fn compute_layout(&mut self, node: Node, available_space: taffy::prelude::Size<taffy::style::AvailableSpace>) -> Result<(), taffy::error::TaffyError> {
        //self.compute_layout(node, taffy::prelude::Size::MAX_CONTENT)
        let size_and_baselines = taffy::prelude::layout_flexbox(
            self,
            node,
            taffy::prelude::Size::NONE,
            available_space.into_options(),
            available_space,
            taffy::layout::SizingMode::InherentSize,
        );
    
        let layout = taffy::prelude::Layout {
            order: 0,
            size: size_and_baselines.size,
            location: taffy::geometry::Point::ZERO,
        };
        *self.layout_mut(node) = layout;
    
        round_layout(self, node, 0., 0.);
    
        Ok(())
    }
}

/// The number of cache entries for each node in the tree
pub const CACHE_SIZE: usize = 7;

/// Layout information for a given [`Node`](crate::node::Node)
///
/// Stored in a [`Taffy`].
pub struct UiNodeData {
    /// The layout strategy used by this node
    pub style: taffy::style::Style,
    /// The results of the layout computation
    pub layout: taffy::prelude::Layout,

    /// Should we try and measure this node?
    pub needs_measure: bool,

    /// The primary cached results of the layout computation
    pub size_cache: [Option<taffy::layout::Cache>; CACHE_SIZE],
}

impl UiNodeData {
    /// Create the data for a new node
    #[must_use]
    pub const fn new(style: taffy::style::Style) -> Self {
        Self { style, size_cache: [None; CACHE_SIZE], layout: taffy::prelude::Layout::new(), needs_measure: false }
    }

    /// Marks a node and all of its parents (recursively) as dirty
    ///
    /// This clears any cached data and signals that the data must be recomputed.
    #[inline]
    pub fn mark_dirty(&mut self) {
        self.size_cache = [None; CACHE_SIZE];
    }
}

fn round_layout(tree: &mut impl LayoutTree, node: Node, abs_x: f32, abs_y: f32) {
    let layout = tree.layout_mut(node);
    let abs_x = abs_x + layout.location.x;
    let abs_y = abs_y + layout.location.y;

    layout.location.x = layout.location.x.round();
    layout.location.y = layout.location.y.round();
    layout.size.width = (abs_x + layout.size.width).round() - abs_x.round();
    layout.size.height = (abs_y + layout.size.height).round() - abs_y.round();

    let child_count = tree.child_count(node);
    for index in 0..child_count {
        let child = tree.child(node, index);
        round_layout(tree, child, abs_x, abs_y);
    }
}