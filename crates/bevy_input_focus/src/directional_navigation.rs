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
//! # Creating a Navigation Graph
//!
//! ## Automatic Navigation (Recommended)
//!
//! The easiest way to set up navigation is to add the [`AutoDirectionalNavigation`] component
//! to your UI entities. The system will automatically compute the nearest neighbor in each direction
//! based on position and size:
//!
//! ```rust,no_run
//! # use bevy_ecs::prelude::*;
//! # use bevy_input_focus::directional_navigation::AutoDirectionalNavigation;
//! # use bevy_ui::Node;
//! fn spawn_button(mut commands: Commands) {
//!     commands.spawn((
//!         Node::default(),
//!         // ... other UI components ...
//!         AutoDirectionalNavigation::default(), // That's it!
//!     ));
//! }
//! ```
//!
//! The navigation graph automatically updates when UI elements move, resize, or are added/removed.
//! Configure the behavior using the [`AutoNavigationConfig`] resource.
//!
//! ## Manual Navigation
//!
//! You can also manually define navigation connections using methods like
//! [`add_edge`](DirectionalNavigationMap::add_edge) and
//! [`add_looping_edges`](DirectionalNavigationMap::add_looping_edges).
//!
//! ## Combining Automatic and Manual
//!
//! Manual edges always take precedence over auto-generated ones, allowing you to use
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

use alloc::vec::Vec;
use bevy_app::prelude::*;
use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    system::SystemParam,
};
use bevy_math::{CompassOctant, Dir2, Vec2};
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
                auto_rebuild_ui_navigation_graph
                    .in_set(UiSystems::PostLayout)
                    .after(bevy_camera::visibility::VisibilitySystems::VisibilityPropagate),
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
/// # Multi-Layer UIs and Z-Index
///
/// **Important**: The automatic navigation system is currently **z-index agnostic** and treats
/// all entities with `AutoDirectionalNavigation` as a flat set, regardless of which UI layer
/// or z-index they belong to. This means navigation may jump between different layers (e.g.,
/// from a background menu to an overlay popup).
///
/// **Workarounds** for multi-layer UIs:
///
/// 1. **Per-layer manual edge generation**: Query entities by layer and call
///    [`auto_generate_navigation_edges()`] separately for each layer:
///    ```rust,ignore
///    for layer in &layers {
///        let nodes: Vec<FocusableArea> = query_layer(layer).collect();
///        auto_generate_navigation_edges(&mut nav_map, &nodes, &config);
///    }
///    ```
///
/// 2. **Manual cross-layer navigation**: Use [`DirectionalNavigationMap::add_edge()`]
///    to define explicit connections between layers (e.g., "Back" button to main menu).
///
/// 3. **Remove component when layer is hidden**: Dynamically add/remove
///    `AutoDirectionalNavigation` based on which layers are currently active.
///
/// See issue [#21679](https://github.com/bevyengine/bevy/issues/21679) for planned
/// improvements to layer-aware automatic navigation.
///
/// # Opting Out
///
/// To disable automatic navigation for specific entities:
///
/// - **Remove the component**: Simply don't add `AutoDirectionalNavigation` to entities
///   that should only use manual navigation edges.
/// - **Dynamically toggle**: Remove/insert the component at runtime to enable/disable
///   automatic navigation as needed.
///
/// Manual edges defined via [`DirectionalNavigationMap`] are completely independent and
/// will continue to work regardless of this component.
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

/// A focusable area with position and size information.
///
/// This struct represents a UI element in the automatic directional navigation system,
/// containing its entity ID, center position, and size for spatial navigation calculations.
///
/// The term "focusable area" avoids confusion with UI [`Node`](bevy_ui::Node) components.
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

// We can't directly implement this for `bevy_ui` types here without circular dependencies,
// so we'll use a more generic approach with separate functions for different component sets.

/// Calculate 1D overlap between two ranges.
///
/// Returns a value between 0.0 (no overlap) and 1.0 (perfect overlap).
fn calculate_1d_overlap(
    origin_pos: f32,
    origin_size: f32,
    candidate_pos: f32,
    candidate_size: f32,
) -> f32 {
    let origin_min = origin_pos - origin_size / 2.0;
    let origin_max = origin_pos + origin_size / 2.0;
    let cand_min = candidate_pos - candidate_size / 2.0;
    let cand_max = candidate_pos + candidate_size / 2.0;

    let overlap = (origin_max.min(cand_max) - origin_min.max(cand_min)).max(0.0);
    let max_overlap = origin_size.min(candidate_size);
    if max_overlap > 0.0 {
        overlap / max_overlap
    } else {
        0.0
    }
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
            calculate_1d_overlap(
                origin_pos.x,
                origin_size.x,
                candidate_pos.x,
                candidate_size.x,
            )
        }
        CompassOctant::East | CompassOctant::West => {
            // Check vertical overlap
            calculate_1d_overlap(
                origin_pos.y,
                origin_size.y,
                candidate_pos.y,
                candidate_size.y,
            )
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
    // Get direction in mathematical coordinates, then flip Y for UI coordinates
    let dir = Dir2::from(octant).as_vec2() * Vec2::new(1.0, -1.0);
    let to_candidate = candidate_pos - origin_pos;
    let distance = to_candidate.length();

    // Check direction first
    // Convert UI coordinates (Y+ = down) to mathematical coordinates (Y+ = up) by flipping Y
    let origin_math = Vec2::new(origin_pos.x, -origin_pos.y);
    let candidate_math = Vec2::new(candidate_pos.x, -candidate_pos.y);
    if !octant.is_in_direction(origin_math, candidate_math) {
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
/// This system runs in `PostUpdate` in the `UiSystems::PostLayout` system set and automatically updates
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
    changed_nodes: Query<
        (),
        (
            With<AutoDirectionalNavigation>,
            Or<(
                Added<AutoDirectionalNavigation>,
                Changed<ComputedNode>,
                Changed<UiGlobalTransform>,
                Changed<InheritedVisibility>,
            )>,
        ),
    >,
    all_nodes: Query<
        (
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
        ),
        With<AutoDirectionalNavigation>,
    >,
) {
    if changed_nodes.is_empty() {
        return;
    }

    let nodes: Vec<FocusableArea> = all_nodes
        .iter()
        .filter_map(|(entity, computed, transform, inherited_visibility)| {
            // Skip hidden or zero-size nodes
            if computed.is_empty() || !inherited_visibility.get() {
                return None;
            }

            let (_scale, _rotation, translation) = transform.to_scale_angle_translation();
            Some(FocusableArea {
                entity,
                position: translation,
                size: computed.size(),
            })
        })
        .collect();

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
    fn test_is_in_direction() {
        let origin = Vec2::new(100.0, 100.0);

        // Node to the north (mathematically up) should have larger Y
        let north_node = Vec2::new(100.0, 150.0);
        assert!(CompassOctant::North.is_in_direction(origin, north_node));
        assert!(!CompassOctant::South.is_in_direction(origin, north_node));

        // Node to the south (mathematically down) should have smaller Y
        let south_node = Vec2::new(100.0, 50.0);
        assert!(CompassOctant::South.is_in_direction(origin, south_node));
        assert!(!CompassOctant::North.is_in_direction(origin, south_node));

        // Node to the east should be in East direction
        let east_node = Vec2::new(150.0, 100.0);
        assert!(CompassOctant::East.is_in_direction(origin, east_node));
        assert!(!CompassOctant::West.is_in_direction(origin, east_node));

        // Node to the northeast (mathematically up-right) should have larger Y, larger X
        let ne_node = Vec2::new(150.0, 150.0);
        assert!(CompassOctant::NorthEast.is_in_direction(origin, ne_node));
        assert!(!CompassOctant::SouthWest.is_in_direction(origin, ne_node));
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
}
