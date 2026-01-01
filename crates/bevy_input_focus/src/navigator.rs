//! Common structs and functions used in both automatic navigators and manual navigators.

use bevy_ecs::prelude::*;
use bevy_math::{CompassOctant, Dir2, Rect, Vec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// Configuration resource for automatic directional navigation and for generating manual
/// navigation edges via [`auto_generate_navigation_edges`](crate::directional_navigation::auto_generate_navigation_edges)
///
/// This resource controls how nodes should be automatically connected in each direction.
#[derive(Resource, Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Resource, Debug, PartialEq, Clone)
)]
pub struct NavigatorConfig {
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

impl Default for NavigatorConfig {
    fn default() -> Self {
        Self {
            min_alignment_factor: 0.0, // Any overlap is acceptable
            max_search_distance: None, // No distance limit
            prefer_aligned: true,      // Prefer well-aligned nodes
        }
    }
}

/// A focusable area with position and size information.
///
/// This struct represents a UI element used during directional navigation,
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
    config: &NavigatorConfig,
) -> f32 {
    // Get direction in mathematical coordinates, then flip Y for UI coordinates
    let dir = Dir2::from(octant).as_vec2() * Vec2::new(1.0, -1.0);
    let to_candidate = candidate_pos - origin_pos;

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

    // Calculate distance between rectangle edges, not centers
    let origin_rect = Rect::from_center_size(origin_pos, origin_size);
    let candidate_rect = Rect::from_center_size(candidate_pos, candidate_size);
    let dx = (candidate_rect.min.x - origin_rect.max.x)
        .max(origin_rect.min.x - candidate_rect.max.x)
        .max(0.0);
    let dy = (candidate_rect.min.y - origin_rect.max.y)
        .max(origin_rect.min.y - candidate_rect.max.y)
        .max(0.0);
    let distance = (dx * dx + dy * dy).sqrt();

    // Check max distance
    if let Some(max_dist) = config.max_search_distance {
        if distance > max_dist {
            return f32::INFINITY;
        }
    }

    // Calculate alignment score using center-to-center direction
    let center_distance = to_candidate.length();
    let alignment = if center_distance > 0.0 {
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

/// Finds the best entity to navigate to from the origin towards the given direction.
///
/// For details on what "best" means here, refer to [`NavigatorConfig`].
pub(crate) fn find_best_candidate(
    origin: &FocusableArea,
    direction: CompassOctant,
    candidates: &[FocusableArea],
    config: &NavigatorConfig,
) -> Option<Entity> {
    // Find best candidate in this direction
    let mut best_candidate = None;
    let mut best_score = f32::INFINITY;

    for candidate in candidates {
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
            direction,
            config,
        );

        if score < best_score {
            best_score = score;
            best_candidate = Some(candidate.entity);
        }
    }

    best_candidate
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
        let config = NavigatorConfig::default();
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
}
