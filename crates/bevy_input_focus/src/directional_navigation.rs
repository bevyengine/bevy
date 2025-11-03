//! A navigation framework for moving between focusable elements based on directional input.
//!
//! While virtual cursors are a common way to navigate UIs with a gamepad (or arrow keys!),
//! they are generally both slow and frustrating to use.
//! Instead, directional inputs should provide a direct way to snap between focusable elements.
//!
//! Like the rest of this crate, the [`InputFocus`] resource is manipulated to track
//! the current focus.
//!
//! Navigating between focusable entities (commonly UI nodes) is done by
//! passing a [`CompassOctant`] into the [`navigate`](DirectionalNavigation::navigate) method
//! from the [`DirectionalNavigation`] system parameter.
//!
//! Under the hood, the [`DirectionalNavigationMap`] stores a directed graph of focusable entities.
//! Each entity can have up to 8 neighbors, one for each [`CompassOctant`], balancing flexibility and required precision.
//!
//! # Automatic Navigation Graph Generation
//!
//! The graph can be built in two ways:
//!
//! 1. **Manual**: Use methods like [`add_edge`](DirectionalNavigationMap::add_edge)
//!    and [`add_looping_edges`](DirectionalNavigationMap::add_looping_edges) to explicitly define connections.
//! 2. **Automatic**: Add the [`AutoDirectionalNavigation`] component to UI entities with
//!    position and size data. The system will automatically compute nearest neighbors in each direction.
//!    Manual edges always take precedence over auto-generated ones.

use alloc::vec::Vec;
use bevy_app::prelude::*;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    system::SystemParam,
};
use bevy_math::{CompassOctant, Vec2};
use bevy_ui::{ComputedNode, UiGlobalTransform, UiSystems};
use thiserror::Error;

use crate::InputFocus;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};

/// A plugin that sets up the directional navigation systems and resources.
#[derive(Default)]
pub struct DirectionalNavigationPlugin;

impl Plugin for DirectionalNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirectionalNavigationMap>()
            .init_resource::<AutoNavigationConfig>()
            .add_systems(
                PostUpdate,
                auto_rebuild_ui_navigation_graph.after(UiSystems::Layout),
            );
    }
}

/// Marker component to enable automatic directional navigation graph generation.
///
/// Simply add this component to your UI entities and the navigation graph will be
/// automatically computed and maintained! The [`DirectionalNavigationPlugin`] includes
/// a built-in system that:
/// - Detects when nodes with this component change position or size
/// - Automatically rebuilds navigation edges based on spatial proximity
/// - Respects manual edges (they always take precedence)
///
///
/// Just add this component to `bevy_ui` entities:
///
/// ```rust
/// # use bevy_ecs::prelude::*;
/// # use bevy_input_focus::directional_navigation::AutoDirectionalNavigation;
/// fn spawn_auto_nav_button(mut commands: Commands) {
///     commands.spawn((
///         // ... Button, Node, etc. ...
///         AutoDirectionalNavigation::default(), // That's it!
///     ));
/// }
/// ```
///
/// The navigation graph updates automatically when nodes move, resize, or are added/removed.
///
/// # Requirements (for `bevy_ui`)
///
/// Entities must also have:
/// - [`ComputedNode`] - for size information
/// - [`UiGlobalTransform`] - for position information
///
/// These are automatically added by `bevy_ui` when you spawn UI entities.
///
/// # Custom UI Systems
///
/// For custom UI frameworks, you can call [`auto_generate_navigation_edges`] directly
/// in your own system instead of using this component.
#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component, Default, Debug, PartialEq, Clone)
)]
pub struct AutoDirectionalNavigation {
    /// Whether to also consider `TabIndex` for navigation order hints.
    /// Currently unused but reserved for future functionality.
    pub respect_tab_order: bool,
}

/// Configuration resource for automatic directional navigation graph generation.
///
/// This resource controls how the automatic navigation system computes which
/// nodes should be connected in each direction.
#[derive(Resource, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Debug, PartialEq, Clone)
)]
pub struct AutoNavigationConfig {
    /// Minimum overlap (0.0-1.0) required in perpendicular axis to consider a node reachable.
    ///
    /// For example, when navigating East/West, nodes must have some vertical overlap.
    /// A value of 0.0 means any overlap is acceptable, while 1.0 means perfect alignment is required.
    pub min_alignment_factor: f32,

    /// Maximum search distance in logical pixels.
    ///
    /// Nodes beyond this distance won't be connected. `None` means unlimited.
    pub max_search_distance: Option<f32>,

    /// Whether to prefer nodes that are more aligned with the exact direction.
    ///
    /// When `true`, nodes that are more directly in line with the requested direction
    /// will be strongly preferred over nodes at an angle.
    pub prefer_aligned: bool,
}

impl Default for AutoNavigationConfig {
    fn default() -> Self {
        Self {
            min_alignment_factor: 0.0, // Any overlap is acceptable
            max_search_distance: None, // No distance limit
            prefer_aligned: true,      // Prefer well-aligned nodes
        }
    }
}

/// The up-to-eight neighbors of a focusable entity, one for each [`CompassOctant`].
#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Default, Debug, PartialEq, Clone)
)]
pub struct NavNeighbors {
    /// The array of neighbors, one for each [`CompassOctant`].
    /// The mapping between array elements and directions is determined by [`CompassOctant::to_index`].
    ///
    /// If no neighbor exists in a given direction, the value will be [`None`].
    /// In most cases, using [`NavNeighbors::set`] and [`NavNeighbors::get`]
    /// will be more ergonomic than directly accessing this array.
    pub neighbors: [Option<Entity>; 8],
}

impl NavNeighbors {
    /// An empty set of neighbors.
    pub const EMPTY: NavNeighbors = NavNeighbors {
        neighbors: [None; 8],
    };

    /// Get the neighbor for a given [`CompassOctant`].
    pub const fn get(&self, octant: CompassOctant) -> Option<Entity> {
        self.neighbors[octant.to_index()]
    }

    /// Set the neighbor for a given [`CompassOctant`].
    pub const fn set(&mut self, octant: CompassOctant, entity: Entity) {
        self.neighbors[octant.to_index()] = Some(entity);
    }
}

/// A resource that stores the traversable graph of focusable entities.
///
/// Each entity can have up to 8 neighbors, one for each [`CompassOctant`].
///
/// To ensure that your graph is intuitive to navigate and generally works correctly, it should be:
///
/// - **Connected**: Every focusable entity should be reachable from every other focusable entity.
/// - **Symmetric**: If entity A is a neighbor of entity B, then entity B should be a neighbor of entity A, ideally in the reverse direction.
/// - **Physical**: The direction of navigation should match the layout of the entities when possible,
///   although looping around the edges of the screen is also acceptable.
/// - **Not self-connected**: An entity should not be a neighbor of itself; use [`None`] instead.
///
/// For now, this graph must be built manually, and the developer is responsible for ensuring that it meets the above criteria.
#[derive(Resource, Debug, Default, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Debug, Default, PartialEq, Clone)
)]
pub struct DirectionalNavigationMap {
    /// A directed graph of focusable entities.
    ///
    /// Pass in the current focus as a key, and get back a collection of up to 8 neighbors,
    /// each keyed by a [`CompassOctant`].
    pub neighbors: EntityHashMap<NavNeighbors>,
}

impl DirectionalNavigationMap {
    /// Adds a new entity to the navigation map, overwriting any existing neighbors for that entity.
    ///
    /// Removes an entity from the navigation map, including all connections to and from it.
    ///
    /// Note that this is an O(n) operation, where n is the number of entities in the map,
    /// as we must iterate over each entity to check for connections to the removed entity.
    ///
    /// If you are removing multiple entities, consider using [`remove_multiple`](Self::remove_multiple) instead.
    pub fn remove(&mut self, entity: Entity) {
        self.neighbors.remove(&entity);

        for node in self.neighbors.values_mut() {
            for neighbor in node.neighbors.iter_mut() {
                if *neighbor == Some(entity) {
                    *neighbor = None;
                }
            }
        }
    }

    /// Removes a collection of entities from the navigation map.
    ///
    /// While this is still an O(n) operation, where n is the number of entities in the map,
    /// it is more efficient than calling [`remove`](Self::remove) multiple times,
    /// as we can check for connections to all removed entities in a single pass.
    ///
    /// An [`EntityHashSet`] must be provided as it is noticeably faster than the standard hasher or a [`Vec`](`alloc::vec::Vec`).
    pub fn remove_multiple(&mut self, entities: EntityHashSet) {
        for entity in &entities {
            self.neighbors.remove(entity);
        }

        for node in self.neighbors.values_mut() {
            for neighbor in node.neighbors.iter_mut() {
                if let Some(entity) = *neighbor {
                    if entities.contains(&entity) {
                        *neighbor = None;
                    }
                }
            }
        }
    }

    /// Completely clears the navigation map, removing all entities and connections.
    pub fn clear(&mut self) {
        self.neighbors.clear();
    }

    /// Adds an edge between two entities in the navigation map.
    /// Any existing edge from A in the provided direction will be overwritten.
    ///
    /// The reverse edge will not be added, so navigation will only be possible in one direction.
    /// If you want to add a symmetrical edge, use [`add_symmetrical_edge`](Self::add_symmetrical_edge) instead.
    pub fn add_edge(&mut self, a: Entity, b: Entity, direction: CompassOctant) {
        self.neighbors
            .entry(a)
            .or_insert(NavNeighbors::EMPTY)
            .set(direction, b);
    }

    /// Adds a symmetrical edge between two entities in the navigation map.
    /// The A -> B path will use the provided direction, while B -> A will use the [`CompassOctant::opposite`] variant.
    ///
    /// Any existing connections between the two entities will be overwritten.
    pub fn add_symmetrical_edge(&mut self, a: Entity, b: Entity, direction: CompassOctant) {
        self.add_edge(a, b, direction);
        self.add_edge(b, a, direction.opposite());
    }

    /// Add symmetrical edges between each consecutive pair of entities in the provided slice.
    ///
    /// Unlike [`add_looping_edges`](Self::add_looping_edges), this method does not loop back to the first entity.
    pub fn add_edges(&mut self, entities: &[Entity], direction: CompassOctant) {
        for pair in entities.windows(2) {
            self.add_symmetrical_edge(pair[0], pair[1], direction);
        }
    }

    /// Add symmetrical edges between each consecutive pair of entities in the provided slice, looping back to the first entity at the end.
    ///
    /// This is useful for creating a circular navigation path between a set of entities, such as a menu.
    pub fn add_looping_edges(&mut self, entities: &[Entity], direction: CompassOctant) {
        self.add_edges(entities, direction);
        if let Some((first_entity, rest)) = entities.split_first() {
            if let Some(last_entity) = rest.last() {
                self.add_symmetrical_edge(*last_entity, *first_entity, direction);
            }
        }
    }

    /// Gets the entity in a given direction from the current focus, if any.
    pub fn get_neighbor(&self, focus: Entity, octant: CompassOctant) -> Option<Entity> {
        self.neighbors
            .get(&focus)
            .and_then(|neighbors| neighbors.get(octant))
    }

    /// Looks up the neighbors of a given entity.
    ///
    /// If the entity is not in the map, [`None`] will be returned.
    /// Note that the set of neighbors is not guaranteed to be non-empty though!
    pub fn get_neighbors(&self, entity: Entity) -> Option<&NavNeighbors> {
        self.neighbors.get(&entity)
    }
}

/// A system parameter for navigating between focusable entities in a directional way.
#[derive(SystemParam, Debug)]
pub struct DirectionalNavigation<'w> {
    /// The currently focused entity.
    pub focus: ResMut<'w, InputFocus>,
    /// The navigation map containing the connections between entities.
    pub map: Res<'w, DirectionalNavigationMap>,
}

impl DirectionalNavigation<'_> {
    /// Navigates to the neighbor in a given direction from the current focus, if any.
    ///
    /// Returns the new focus if successful.
    /// Returns an error if there is no focus set or if there is no neighbor in the requested direction.
    ///
    /// If the result was `Ok`, the [`InputFocus`] resource is updated to the new focus as part of this method call.
    pub fn navigate(
        &mut self,
        direction: CompassOctant,
    ) -> Result<Entity, DirectionalNavigationError> {
        if let Some(current_focus) = self.focus.0 {
            if let Some(new_focus) = self.map.get_neighbor(current_focus, direction) {
                self.focus.set(new_focus);
                Ok(new_focus)
            } else {
                Err(DirectionalNavigationError::NoNeighborInDirection {
                    current_focus,
                    direction,
                })
            }
        } else {
            Err(DirectionalNavigationError::NoFocus)
        }
    }
}

/// An error that can occur when navigating between focusable entities using [directional navigation](crate::directional_navigation).
#[derive(Debug, PartialEq, Clone, Error)]
pub enum DirectionalNavigationError {
    /// No focusable entity is currently set.
    #[error("No focusable entity is currently set.")]
    NoFocus,
    /// No neighbor in the requested direction.
    #[error("No neighbor from {current_focus} in the {direction:?} direction.")]
    NoNeighborInDirection {
        /// The entity that was the focus when the error occurred.
        current_focus: Entity,
        /// The direction in which the navigation was attempted.
        direction: CompassOctant,
    },
}

/// A navigation node with position and size information.
///
/// This struct represents a UI element in the automatic directional navigation system,
/// containing its entity ID, center position, and size for spatial navigation calculations.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
pub struct NavigationNode {
    /// The entity identifier for this navigation node.
    pub entity: Entity,
    /// The center position of the node in global coordinates.
    pub position: Vec2,
    /// The size (width, height) of the node.
    pub size: Vec2,
}

/// Trait for extracting position and size from UI node components.
///
/// This allows the auto-navigation system to work with different UI implementations
/// as long as they can provide position and size information.
pub trait NavigableNode {
    /// Returns the center position and size of the node in global coordinates.
    fn get_bounds(&self) -> (Vec2, Vec2);
}

// We can't directly implement this for `bevy_ui` types here without circular dependencies,
// so we'll use a more generic approach with separate functions for different component sets.

/// Convert `CompassOctant` to a unit direction vector.
///
fn octant_to_direction(octant: CompassOctant) -> Vec2 {
    use CompassOctant::*;
    match octant {
        North => Vec2::new(0.0, -1.0),
        NorthEast => Vec2::new(1.0, -1.0).normalize(),
        East => Vec2::new(1.0, 0.0),
        SouthEast => Vec2::new(1.0, 1.0).normalize(),
        South => Vec2::new(0.0, 1.0),
        SouthWest => Vec2::new(-1.0, 1.0).normalize(),
        West => Vec2::new(-1.0, 0.0),
        NorthWest => Vec2::new(-1.0, -1.0).normalize(),
    }
}

/// Check if node `candidate` is in direction `octant` from node `origin`.
///
/// This uses a cone-based check: the vector from origin to candidate
/// must have a positive dot product with the direction vector.
fn is_in_direction(origin_pos: Vec2, candidate_pos: Vec2, octant: CompassOctant) -> bool {
    let dir = octant_to_direction(octant);
    let to_candidate = candidate_pos - origin_pos;

    // Check if the candidate is generally in the right direction
    // Use a cone-based check: dot product should be positive
    to_candidate.dot(dir) > 0.0
}

/// Calculate the overlap factor between two nodes in the perpendicular axis.
///
/// Returns a value between 0.0 (no overlap) and 1.0 (perfect overlap).
/// For diagonal directions, always returns 1.0.
fn calculate_overlap(
    origin_pos: Vec2,
    origin_size: Vec2,
    candidate_pos: Vec2,
    candidate_size: Vec2,
    octant: CompassOctant,
) -> f32 {
    match octant {
        CompassOctant::North | CompassOctant::South => {
            // Check horizontal overlap
            let origin_left = origin_pos.x - origin_size.x / 2.0;
            let origin_right = origin_pos.x + origin_size.x / 2.0;
            let cand_left = candidate_pos.x - candidate_size.x / 2.0;
            let cand_right = candidate_pos.x + candidate_size.x / 2.0;

            let overlap = (origin_right.min(cand_right) - origin_left.max(cand_left)).max(0.0);
            let max_overlap = origin_size.x.min(candidate_size.x);
            if max_overlap > 0.0 {
                overlap / max_overlap
            } else {
                0.0
            }
        }
        CompassOctant::East | CompassOctant::West => {
            // Check vertical overlap
            let origin_bottom = origin_pos.y - origin_size.y / 2.0;
            let origin_top = origin_pos.y + origin_size.y / 2.0;
            let cand_bottom = candidate_pos.y - candidate_size.y / 2.0;
            let cand_top = candidate_pos.y + candidate_size.y / 2.0;

            let overlap = (origin_top.min(cand_top) - origin_bottom.max(cand_bottom)).max(0.0);
            let max_overlap = origin_size.y.min(candidate_size.y);
            if max_overlap > 0.0 {
                overlap / max_overlap
            } else {
                0.0
            }
        }
        // Diagonal directions don't require strict overlap
        _ => 1.0,
    }
}

/// Score a candidate node for navigation in a given direction.
///
/// Lower score is better. Returns `f32::INFINITY` for unreachable nodes.
fn score_candidate(
    origin_pos: Vec2,
    origin_size: Vec2,
    candidate_pos: Vec2,
    candidate_size: Vec2,
    octant: CompassOctant,
    config: &AutoNavigationConfig,
) -> f32 {
    let dir = octant_to_direction(octant);
    let to_candidate = candidate_pos - origin_pos;
    let distance = to_candidate.length();

    // Check direction first
    if !is_in_direction(origin_pos, candidate_pos, octant) {
        return f32::INFINITY;
    }

    // Check overlap for cardinal directions
    let overlap_factor = calculate_overlap(
        origin_pos,
        origin_size,
        candidate_pos,
        candidate_size,
        octant,
    );

    if overlap_factor < config.min_alignment_factor {
        return f32::INFINITY;
    }

    // Check max distance
    if let Some(max_dist) = config.max_search_distance {
        if distance > max_dist {
            return f32::INFINITY;
        }
    }

    // Calculate alignment score
    let alignment = if distance > 0.0 {
        to_candidate.normalize().dot(dir).max(0.0)
    } else {
        1.0
    };

    // Combine distance and alignment
    // Prefer aligned nodes by penalizing misalignment
    let alignment_penalty = if config.prefer_aligned {
        (1.0 - alignment) * distance * 2.0 // Misalignment scales with distance
    } else {
        0.0
    };

    distance + alignment_penalty
}

/// Automatically generates directional navigation edges for a collection of nodes.
///
/// This function takes a slice of navigation nodes with their positions and sizes, and populates
/// the navigation map with edges to the nearest neighbor in each compass direction.
/// Manual edges in the map are preserved and not overwritten.
///
/// # Arguments
///
/// * `nav_map` - The navigation map to populate
/// * `nodes` - A slice of [`NavigationNode`] structs containing entity, position, and size data
/// * `config` - Configuration for the auto-generation algorithm
///
/// # Example
///
/// ```rust
/// # use bevy_input_focus::directional_navigation::*;
/// # use bevy_ecs::entity::Entity;
/// # use bevy_math::Vec2;
/// let mut nav_map = DirectionalNavigationMap::default();
/// let config = AutoNavigationConfig::default();
///
/// let nodes = vec![
///     NavigationNode { entity: Entity::PLACEHOLDER, position: Vec2::new(100.0, 100.0), size: Vec2::new(50.0, 50.0) },
///     NavigationNode { entity: Entity::PLACEHOLDER, position: Vec2::new(200.0, 100.0), size: Vec2::new(50.0, 50.0) },
/// ];
///
/// auto_generate_navigation_edges(&mut nav_map, &nodes, &config);
/// ```
pub fn auto_generate_navigation_edges(
    nav_map: &mut DirectionalNavigationMap,
    nodes: &[NavigationNode],
    config: &AutoNavigationConfig,
) {
    // For each node, find best neighbor in each direction
    for origin in nodes {
        for octant in [
            CompassOctant::North,
            CompassOctant::NorthEast,
            CompassOctant::East,
            CompassOctant::SouthEast,
            CompassOctant::South,
            CompassOctant::SouthWest,
            CompassOctant::West,
            CompassOctant::NorthWest,
        ] {
            // Skip if manual edge already exists (check inline to avoid borrow issues)
            if nav_map
                .get_neighbors(origin.entity)
                .and_then(|neighbors| neighbors.get(octant))
                .is_some()
            {
                continue; // Respect manual override
            }

            // Find best candidate in this direction
            let mut best_candidate = None;
            let mut best_score = f32::INFINITY;

            for candidate in nodes {
                // Skip self
                if candidate.entity == origin.entity {
                    continue;
                }

                // Score the candidate
                let score = score_candidate(
                    origin.position,
                    origin.size,
                    candidate.position,
                    candidate.size,
                    octant,
                    config,
                );

                if score < best_score {
                    best_score = score;
                    best_candidate = Some(candidate.entity);
                }
            }

            // Add edge if we found a valid candidate
            if let Some(neighbor) = best_candidate {
                nav_map.add_edge(origin.entity, neighbor, octant);
            }
        }
    }
}

/// Built-in system that automatically rebuilds the navigation graph for `bevy_ui` nodes.
///
/// This system runs in `PostUpdate` after `UiSystems::Layout` and automatically updates
/// the navigation graph when nodes with [`AutoDirectionalNavigation`] component change
/// their position or size.
///
/// # How it works
///
/// 1. Detects nodes with [`AutoDirectionalNavigation`] that have changed
/// 2. Extracts position/size from [`ComputedNode`] and [`UiGlobalTransform`]
/// 3. Calls [`auto_generate_navigation_edges`] to rebuild connections
///
/// This system is automatically added by [`DirectionalNavigationPlugin`], so users
/// only need to add the [`AutoDirectionalNavigation`] component to their UI entities.
///
/// # Note
///
/// This system only works with `bevy_ui` nodes. For custom UI systems, call
/// [`auto_generate_navigation_edges`] directly in your own system.
fn auto_rebuild_ui_navigation_graph(
    mut directional_nav_map: ResMut<DirectionalNavigationMap>,
    config: Res<AutoNavigationConfig>,
    // Query nodes that changed
    auto_nav_nodes: Query<
        (Entity, &ComputedNode, &UiGlobalTransform),
        (
            With<AutoDirectionalNavigation>,
            Or<(Changed<ComputedNode>, Changed<UiGlobalTransform>)>,
        ),
    >,
    // Also need all auto-nav nodes for context (not just changed ones)
    all_auto_nav_nodes: Query<
        (Entity, &ComputedNode, &UiGlobalTransform),
        With<AutoDirectionalNavigation>,
    >,
) {
    // Only rebuild if something changed
    if auto_nav_nodes.is_empty() {
        return;
    }

    // Collect all nodes with their positions and sizes
    let nodes: Vec<NavigationNode> = all_auto_nav_nodes
        .iter()
        .filter(|(_, computed, _)| !computed.is_empty())
        .map(|(entity, computed, transform)| {
            // Extract center position from transform
            let (_scale, _rotation, translation) = transform.to_scale_angle_translation();
            let size = computed.size();
            NavigationNode {
                entity,
                position: translation,
                size,
            }
        })
        .collect();

    // Use the automatic edge generation function
    auto_generate_navigation_edges(&mut directional_nav_map, &nodes, &config);
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use bevy_ecs::system::RunSystemOnce;

    use super::*;

    #[test]
    fn setting_and_getting_nav_neighbors() {
        let mut neighbors = NavNeighbors::EMPTY;
        assert_eq!(neighbors.get(CompassOctant::SouthEast), None);

        neighbors.set(CompassOctant::SouthEast, Entity::PLACEHOLDER);

        for i in 0..8 {
            if i == CompassOctant::SouthEast.to_index() {
                assert_eq!(
                    neighbors.get(CompassOctant::SouthEast),
                    Some(Entity::PLACEHOLDER)
                );
            } else {
                assert_eq!(neighbors.get(CompassOctant::from_index(i).unwrap()), None);
            }
        }
    }

    #[test]
    fn simple_set_and_get_navmap() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::SouthEast);

        assert_eq!(map.get_neighbor(a, CompassOctant::SouthEast), Some(b));
        assert_eq!(
            map.get_neighbor(b, CompassOctant::SouthEast.opposite()),
            None
        );
    }

    #[test]
    fn symmetrical_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_symmetrical_edge(a, b, CompassOctant::North);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::South), Some(a));
    }

    #[test]
    fn remove_nodes() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::North);
        map.add_edge(b, a, CompassOctant::South);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::South), Some(a));

        map.remove(b);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::South), None);
    }

    #[test]
    fn remove_multiple_nodes() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edge(a, b, CompassOctant::North);
        map.add_edge(b, a, CompassOctant::South);
        map.add_edge(b, c, CompassOctant::East);
        map.add_edge(c, b, CompassOctant::West);

        let mut to_remove = EntityHashSet::default();
        to_remove.insert(b);
        to_remove.insert(c);

        map.remove_multiple(to_remove);

        assert_eq!(map.get_neighbor(a, CompassOctant::North), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::South), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::East), None);
        assert_eq!(map.get_neighbor(c, CompassOctant::West), None);
    }

    #[test]
    fn edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_edges(&[a, b, c], CompassOctant::East);

        assert_eq!(map.get_neighbor(a, CompassOctant::East), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::East), Some(c));
        assert_eq!(map.get_neighbor(c, CompassOctant::East), None);

        assert_eq!(map.get_neighbor(a, CompassOctant::West), None);
        assert_eq!(map.get_neighbor(b, CompassOctant::West), Some(a));
        assert_eq!(map.get_neighbor(c, CompassOctant::West), Some(b));
    }

    #[test]
    fn looping_edges() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_looping_edges(&[a, b, c], CompassOctant::East);

        assert_eq!(map.get_neighbor(a, CompassOctant::East), Some(b));
        assert_eq!(map.get_neighbor(b, CompassOctant::East), Some(c));
        assert_eq!(map.get_neighbor(c, CompassOctant::East), Some(a));

        assert_eq!(map.get_neighbor(a, CompassOctant::West), Some(c));
        assert_eq!(map.get_neighbor(b, CompassOctant::West), Some(a));
        assert_eq!(map.get_neighbor(c, CompassOctant::West), Some(b));
    }

    #[test]
    fn nav_with_system_param() {
        let mut world = World::new();
        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn_empty().id();

        let mut map = DirectionalNavigationMap::default();
        map.add_looping_edges(&[a, b, c], CompassOctant::East);

        world.insert_resource(map);

        let mut focus = InputFocus::default();
        focus.set(a);
        world.insert_resource(focus);

        assert_eq!(world.resource::<InputFocus>().get(), Some(a));

        fn navigate_east(mut nav: DirectionalNavigation) {
            nav.navigate(CompassOctant::East).unwrap();
        }

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(b));

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(c));

        world.run_system_once(navigate_east).unwrap();
        assert_eq!(world.resource::<InputFocus>().get(), Some(a));
    }

    // Tests for automatic navigation helpers
    #[test]
    fn test_octant_to_direction() {
        let north = octant_to_direction(CompassOctant::North);
        assert_eq!(north, Vec2::new(0.0, -1.0));

        let east = octant_to_direction(CompassOctant::East);
        assert_eq!(east, Vec2::new(1.0, 0.0));

        let south = octant_to_direction(CompassOctant::South);
        assert_eq!(south, Vec2::new(0.0, 1.0));

        let west = octant_to_direction(CompassOctant::West);
        assert_eq!(west, Vec2::new(-1.0, 0.0));
    }

    #[test]
    fn test_is_in_direction() {
        let origin = Vec2::new(100.0, 100.0);

        // Node to the north (up on screen) should have smaller Y
        let north_node = Vec2::new(100.0, 50.0);
        assert!(is_in_direction(origin, north_node, CompassOctant::North));
        assert!(!is_in_direction(origin, north_node, CompassOctant::South));

        // Node to the south (down on screen) should have larger Y
        let south_node = Vec2::new(100.0, 150.0);
        assert!(is_in_direction(origin, south_node, CompassOctant::South));
        assert!(!is_in_direction(origin, south_node, CompassOctant::North));

        // Node to the east should be in East direction
        let east_node = Vec2::new(150.0, 100.0);
        assert!(is_in_direction(origin, east_node, CompassOctant::East));
        assert!(!is_in_direction(origin, east_node, CompassOctant::West));

        // Node to the northeast (up-right on screen) should have smaller Y, larger X
        let ne_node = Vec2::new(150.0, 50.0);
        assert!(is_in_direction(origin, ne_node, CompassOctant::NorthEast));
        assert!(!is_in_direction(origin, ne_node, CompassOctant::SouthWest));
    }

    #[test]
    fn test_calculate_overlap_horizontal() {
        let origin_pos = Vec2::new(100.0, 100.0);
        let origin_size = Vec2::new(50.0, 50.0);

        // Fully overlapping node to the north
        let north_pos = Vec2::new(100.0, 200.0);
        let north_size = Vec2::new(50.0, 50.0);
        let overlap = calculate_overlap(
            origin_pos,
            origin_size,
            north_pos,
            north_size,
            CompassOctant::North,
        );
        assert_eq!(overlap, 1.0); // Full overlap

        // Partially overlapping node to the north
        let north_pos = Vec2::new(110.0, 200.0);
        let partial_overlap = calculate_overlap(
            origin_pos,
            origin_size,
            north_pos,
            north_size,
            CompassOctant::North,
        );
        assert!(partial_overlap > 0.0 && partial_overlap < 1.0);

        // No overlap
        let north_pos = Vec2::new(200.0, 200.0);
        let no_overlap = calculate_overlap(
            origin_pos,
            origin_size,
            north_pos,
            north_size,
            CompassOctant::North,
        );
        assert_eq!(no_overlap, 0.0);
    }

    #[test]
    fn test_score_candidate() {
        let config = AutoNavigationConfig::default();
        let origin_pos = Vec2::new(100.0, 100.0);
        let origin_size = Vec2::new(50.0, 50.0);

        // Node directly to the north (up on screen = smaller Y)
        let north_pos = Vec2::new(100.0, 0.0);
        let north_size = Vec2::new(50.0, 50.0);
        let north_score = score_candidate(
            origin_pos,
            origin_size,
            north_pos,
            north_size,
            CompassOctant::North,
            &config,
        );
        assert!(north_score < f32::INFINITY);
        assert!(north_score < 150.0); // Should be close to the distance (100)

        // Node in opposite direction (should be unreachable)
        let south_pos = Vec2::new(100.0, 200.0);
        let south_size = Vec2::new(50.0, 50.0);
        let invalid_score = score_candidate(
            origin_pos,
            origin_size,
            south_pos,
            south_size,
            CompassOctant::North,
            &config,
        );
        assert_eq!(invalid_score, f32::INFINITY);

        // Closer node should have better score than farther node
        let close_pos = Vec2::new(100.0, 50.0);
        let far_pos = Vec2::new(100.0, -100.0);
        let close_score = score_candidate(
            origin_pos,
            origin_size,
            close_pos,
            north_size,
            CompassOctant::North,
            &config,
        );
        let far_score = score_candidate(
            origin_pos,
            origin_size,
            far_pos,
            north_size,
            CompassOctant::North,
            &config,
        );
        assert!(close_score < far_score);
    }

    #[test]
    fn test_auto_generate_navigation_edges() {
        let mut nav_map = DirectionalNavigationMap::default();
        let config = AutoNavigationConfig::default();

        // Create a 2x2 grid of nodes (using UI coordinates: smaller Y = higher on screen)
        let node_a = Entity::from_bits(1); // Top-left
        let node_b = Entity::from_bits(2); // Top-right
        let node_c = Entity::from_bits(3); // Bottom-left
        let node_d = Entity::from_bits(4); // Bottom-right

        let nodes = vec![
            NavigationNode {
                entity: node_a,
                position: Vec2::new(0.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Top-left
            NavigationNode {
                entity: node_b,
                position: Vec2::new(100.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Top-right
            NavigationNode {
                entity: node_c,
                position: Vec2::new(0.0, 100.0),
                size: Vec2::new(50.0, 50.0),
            }, // Bottom-left
            NavigationNode {
                entity: node_d,
                position: Vec2::new(100.0, 100.0),
                size: Vec2::new(50.0, 50.0),
            }, // Bottom-right
        ];

        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);

        // Test horizontal navigation
        assert_eq!(
            nav_map.get_neighbor(node_a, CompassOctant::East),
            Some(node_b)
        );
        assert_eq!(
            nav_map.get_neighbor(node_b, CompassOctant::West),
            Some(node_a)
        );

        // Test vertical navigation
        assert_eq!(
            nav_map.get_neighbor(node_a, CompassOctant::South),
            Some(node_c)
        );
        assert_eq!(
            nav_map.get_neighbor(node_c, CompassOctant::North),
            Some(node_a)
        );

        // Test diagonal navigation
        assert_eq!(
            nav_map.get_neighbor(node_a, CompassOctant::SouthEast),
            Some(node_d)
        );
    }

    #[test]
    fn test_auto_generate_respects_manual_edges() {
        let mut nav_map = DirectionalNavigationMap::default();
        let config = AutoNavigationConfig::default();

        let node_a = Entity::from_bits(1);
        let node_b = Entity::from_bits(2);
        let node_c = Entity::from_bits(3);

        // Manually set an edge from A to C (skipping B)
        nav_map.add_edge(node_a, node_c, CompassOctant::East);

        let nodes = vec![
            NavigationNode {
                entity: node_a,
                position: Vec2::new(0.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            },
            NavigationNode {
                entity: node_b,
                position: Vec2::new(50.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Closer
            NavigationNode {
                entity: node_c,
                position: Vec2::new(100.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            },
        ];

        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);

        // The manual edge should be preserved, even though B is closer
        assert_eq!(
            nav_map.get_neighbor(node_a, CompassOctant::East),
            Some(node_c)
        );
    }
}
