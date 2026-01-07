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
//! from the [`DirectionalNavigation`] system parameter. Under the hood, an entity is found
//! automatically via brute force search in the desired [`CompassOctant`] direction.
//!
//! If some manual navigation is desired, a [`DirectionalNavigationMap`] will override the brute force
//! search in a direction for a given entity. The [`DirectionalNavigationMap`] stores a directed graph
//! of focusable entities. Each entity can have up to 8 neighbors, one for each [`CompassOctant`],
//! balancing flexibility and required precision.
//!
//! # Setting up Directional Navigation
//!
//! ## Automatic Navigation (Recommended)
//!
//! The easiest way to set up navigation is to add the `AutoDirectionalNavigation` component
//! to your UI entities. This component is available in the `bevy_ui` crate. If you choose to
//! include automatic navigation, you should also use the `AutoDirectionalNavigator` system parameter
//! in that crate instead of [`DirectionalNavigation`].
//!
//! ## Manual Navigation
//!
//! You can also manually define navigation connections using methods like
//! [`add_edge`](DirectionalNavigationMap::add_edge) and
//! [`add_looping_edges`](DirectionalNavigationMap::add_looping_edges).
//!
//! ## Combining Automatic and Manual
//!
//! Following manual edges always take precedence, allowing you to use
//! automatic navigation for most UI elements while overriding specific connections for
//! special cases like wrapping menus or cross-layer navigation.
//!
//! ## When to Use Manual Navigation
//!
//! While automatic navigation is recommended for most use cases, manual navigation provides:
//!
//! - **Precise control**: Define exact navigation flow, including non-obvious connections like looping edges
//! - **Cross-layer navigation**: Connect elements across different UI layers or z-index levels
//! - **Custom behavior**: Implement domain-specific navigation patterns (e.g., spreadsheet-style wrapping)

use crate::{navigator::find_best_candidate, InputFocus};
use bevy_app::prelude::*;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    system::SystemParam,
};

use thiserror::Error;
use bevy_math::{CompassOctant, Vec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{prelude::*, Reflect};

/// A plugin that sets up the directional navigation resources.
#[derive(Default)]
pub struct DirectionalNavigationPlugin;

impl Plugin for DirectionalNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirectionalNavigationMap>()
            .init_resource::<AutoNavigationConfig>();
    }
}

/// Configuration resource for automatic directional navigation and for generating manual
/// navigation edges via [`auto_generate_navigation_edges`]
///
/// This resource controls how nodes should be automatically connected in each direction.
#[derive(Resource, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Debug, PartialEq, Clone)
)]
pub struct AutoNavigationConfig {
    /// Minimum overlap ratio (0.0-1.0) required along the perpendicular axis for cardinal directions.
    ///
    /// This parameter controls how much two UI elements must overlap in the perpendicular direction
    /// to be considered reachable neighbors. It only applies to cardinal directions (`North`, `South`, `East`, `West`);
    /// diagonal directions (`NorthEast`, `SouthEast`, etc.) ignore this requirement entirely.
    ///
    /// # Calculation
    ///
    /// The overlap factor is calculated as:
    /// ```text
    /// overlap_factor = actual_overlap / min(origin_size, candidate_size)
    /// ```
    ///
    /// For East/West navigation, this measures vertical overlap:
    /// - `actual_overlap` = overlapping height between the two elements
    /// - Sizes are the heights of the origin and candidate
    ///
    /// For North/South navigation, this measures horizontal overlap:
    /// - `actual_overlap` = overlapping width between the two elements
    /// - Sizes are the widths of the origin and candidate
    ///
    /// # Examples
    ///
    /// - `0.0` (default): Any overlap is sufficient. Even if elements barely touch, they can be neighbors.
    /// - `0.5`: Elements must overlap by at least 50% of the smaller element's size.
    /// - `1.0`: Perfect alignment required. The smaller element must be completely within the bounds
    ///   of the larger element along the perpendicular axis.
    ///
    /// # Use Cases
    ///
    /// - **Sparse/irregular layouts** (e.g., star constellations): Use `0.0` to allow navigation
    ///   between elements that don't directly align.
    /// - **Grid layouts**: Use `0.5` or higher to ensure navigation only connects elements in
    ///   the same row or column.
    /// - **Strict alignment**: Use `1.0` to require perfect alignment, though this may result
    ///   in disconnected navigation graphs if elements aren't precisely aligned.
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

/// A resource that stores the manually specified traversable graph of focusable entities.
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
/// This graph must be built and maintained manually, and the developer is responsible for ensuring that it meets the above criteria.
/// Notably, if the developer adds or removes the navigability of an entity, the developer should update the map as necessary.
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
    /// The directional navigation map containing manually defined connections between entities.
    pub map: Res<'w, DirectionalNavigationMap>,
}

impl<'w> DirectionalNavigation<'w> {
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
            // Respect manual edges first
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

/// A focusable area with position and size information.
///
/// This struct represents a UI element used during directional navigation,
/// containing its entity ID, center position, and size for spatial navigation calculations.
///
/// The term "focusable area" avoids confusion with UI `Node` components in `bevy_ui`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Clone)
)]
pub struct FocusableArea {
    /// The entity identifier for this focusable area.
    pub entity: Entity,
    /// The center position in global coordinates.
    pub position: Vec2,
    /// The size (width, height) of the area.
    pub size: Vec2,
}

/// Trait for extracting position and size from navigable UI components.
///
/// This allows the auto-navigation system to work with different UI implementations
/// as long as they can provide position and size information.
pub trait Navigable {
    /// Returns the center position and size in global coordinates.
    fn get_bounds(&self) -> (Vec2, Vec2);
}

/// Automatically generates directional navigation edges for a collection of nodes.
///
/// This function takes a slice of navigation nodes with their positions and sizes, and populates
/// the navigation map with edges to the nearest neighbor in each compass direction.
/// Manual edges already in the map are preserved and not overwritten.
///
/// # Arguments
///
/// * `nav_map` - The navigation map to populate
/// * `nodes` - A slice of [`FocusableArea`] structs containing entity, position, and size data
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
///     FocusableArea { entity: Entity::PLACEHOLDER, position: Vec2::new(100.0, 100.0), size: Vec2::new(50.0, 50.0) },
///     FocusableArea { entity: Entity::PLACEHOLDER, position: Vec2::new(200.0, 100.0), size: Vec2::new(50.0, 50.0) },
/// ];
///
/// auto_generate_navigation_edges(&mut nav_map, &nodes, &config);
/// ```
pub fn auto_generate_navigation_edges(
    nav_map: &mut DirectionalNavigationMap,
    nodes: &[FocusableArea],
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
            let best_candidate = find_best_candidate(origin, octant, nodes, config);

            // Add edge if we found a valid candidate
            if let Some(neighbor) = best_candidate {
                nav_map.add_edge(origin.entity, neighbor, octant);
            }
        }
    }
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
    fn manual_nav_with_system_param() {
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

        let config = AutoNavigationConfig::default();
        world.insert_resource(config);

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
            FocusableArea {
                entity: node_a,
                position: Vec2::new(0.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Top-left
            FocusableArea {
                entity: node_b,
                position: Vec2::new(100.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Top-right
            FocusableArea {
                entity: node_c,
                position: Vec2::new(0.0, 100.0),
                size: Vec2::new(50.0, 50.0),
            }, // Bottom-left
            FocusableArea {
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
            FocusableArea {
                entity: node_a,
                position: Vec2::new(0.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            },
            FocusableArea {
                entity: node_b,
                position: Vec2::new(50.0, 0.0),
                size: Vec2::new(50.0, 50.0),
            }, // Closer
            FocusableArea {
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

    #[test]
    fn test_edge_distance_vs_center_distance() {
        let mut nav_map = DirectionalNavigationMap::default();
        let config = AutoNavigationConfig::default();

        let left = Entity::from_bits(1);
        let wide_top = Entity::from_bits(2);
        let bottom = Entity::from_bits(3);

        let left_node = FocusableArea {
            entity: left,
            position: Vec2::new(100.0, 200.0),
            size: Vec2::new(100.0, 100.0),
        };

        let wide_top_node = FocusableArea {
            entity: wide_top,
            position: Vec2::new(350.0, 150.0),
            size: Vec2::new(300.0, 80.0),
        };

        let bottom_node = FocusableArea {
            entity: bottom,
            position: Vec2::new(270.0, 300.0),
            size: Vec2::new(100.0, 80.0),
        };

        let nodes = vec![left_node, wide_top_node, bottom_node];

        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);

        assert_eq!(
            nav_map.get_neighbor(left, CompassOctant::East),
            Some(wide_top),
            "Should navigate to wide_top not bottom, even though bottom's center is closer."
        );
    }
}
