//! The easiest way to set up navigation is to add the [`AutoDirectionalNavigation`] component
//! to your UI entities. The system will automatically compute the nearest neighbor in each direction
//! based on position and size:
//!
//! ```rust,no_run
//! # use bevy_ecs::prelude::*;
//! # use bevy_ui::Node;
//! # use bevy_ui::directional_navigation::AutoDirectionalNavigation;
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

use crate::{ComputedNode, UiGlobalTransform, UiSystems};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::visibility::InheritedVisibility;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Added, Changed, Or, With},
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Query, Res, ResMut},
};
use bevy_input_focus::directional_navigation::{DirectionalNavigationMap, FocusableArea};
use bevy_math::{CompassOctant, Dir2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

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
/// # use bevy_ui::directional_navigation::AutoDirectionalNavigation;
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
/// # Requirements
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
#[derive(Component, Default, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct AutoDirectionalNavigation {
    /// Whether to also consider `TabIndex` for navigation order hints.
    /// Currently unused but reserved for future functionality.
    pub respect_tab_order: bool,
}

/// Configuration resource for automatic directional navigation graph generation.
///
/// This resource controls how the automatic navigation system computes which
/// nodes should be connected in each direction.
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource, Debug, PartialEq, Clone)]
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
    if let Some(max_dist) = config.max_search_distance
        && distance > max_dist
    {
        return f32::INFINITY;
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
/// # use bevy_ui::directional_navigation::{AutoNavigationConfig, auto_generate_navigation_edges};
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

/// A plugin that sets up the automatic directional navigation systems and resources.
#[derive(Default)]
pub struct AutoDirectionalNavigationPlugin;

impl Plugin for AutoDirectionalNavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoNavigationConfig>().add_systems(
            PostUpdate,
            auto_rebuild_ui_navigation_graph
                .in_set(UiSystems::PostLayout)
                .after(bevy_camera::visibility::VisibilitySystems::VisibilityPropagate),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
