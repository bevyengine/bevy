//! Functions used by navigators to determine where to go next.
use crate::directional_navigation::{AutoNavigationConfig, FocusableArea};
use bevy_ecs::prelude::*;
use bevy_math::{CompassOctant, Dir2, Rect, Vec2};

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

/// Calculate 2D overlap of the 2D-candidate within a quadrant of the coordinate plane.
///
/// Returns a value between 0.0 (no overlap) and 1.0 (perfect overlap).
/// The overlap returned is the proportion of the candidate's area that lies within
/// the quadrant, defined by `x_clamp` and `y_clamp`, by the candidate's original area.
/// If the candidate's original area is <= 0.0, 0.0 (no overlap) is returned.
///
/// The x and y clamps must be unbounded, meaning that the minimum clamp must be `f32::NEG_INFINITY`
/// or the maximum clamp must be `f32::INFINITY`. The clamps are specified in UI coordinates, i.e.
/// y increases downwards. For example, an `x_clamp` between `30.` and `f32::INFINITY` alongside
/// a `y_clamp` between `f32::NEG_INFINITY` and `20.` specifies a quadrant to the North
/// and East with its center at `Vec2::new(30., 20.)`.
fn calculate_unbounded_2d_overlap(
    x_clamp: impl Fn(f32) -> f32,
    y_clamp: impl Fn(f32) -> f32,
    candidate_pos: Vec2,
    candidate_size: Vec2,
) -> f32 {
    let area = candidate_size.x * candidate_size.y;
    if area <= 0.0 {
        return 0.0;
    }
    let cand_min = candidate_pos - candidate_size / 2.0;
    let cand_max = candidate_pos + candidate_size / 2.0;

    let cand_clamped_size = Vec2::new(
        x_clamp(cand_max.x) - x_clamp(cand_min.x),
        y_clamp(cand_max.y) - y_clamp(cand_min.y),
    );
    let clamped_area = (cand_clamped_size.x * cand_clamped_size.y).max(0.0);

    // Using area as the denominator here only makes sense because the x and y clamps are unbounded
    clamped_area / area
}

/// Calculate the overlap factor between two nodes.
///
/// Returns a value between 0.0 (no overlap) and 1.0 (perfect overlap).
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
        CompassOctant::NorthEast => {
            // Check for overlap in the NorthEast quadrant with its center at the origin's NE Corner
            let origin_pos_north_east = origin_pos + (origin_size / 2.) * Vec2::new(1., -1.);

            calculate_unbounded_2d_overlap(
                |x| x.clamp(origin_pos_north_east.x, f32::INFINITY),
                |y| y.clamp(f32::NEG_INFINITY, origin_pos_north_east.y),
                candidate_pos,
                candidate_size,
            )
        }
        CompassOctant::SouthEast => {
            // Check for overlap in the SouthEast quadrant with its center at the origin's SE Corner
            let origin_pos_south_east = origin_pos + (origin_size / 2.);

            calculate_unbounded_2d_overlap(
                |x| x.clamp(origin_pos_south_east.x, f32::INFINITY),
                |y| y.clamp(origin_pos_south_east.y, f32::INFINITY),
                candidate_pos,
                candidate_size,
            )
        }
        CompassOctant::SouthWest => {
            // Check for overlap in the SouthWest quadrant with its center at the origin's SW Corner
            let origin_pos_south_west = origin_pos + (origin_size / 2.) * Vec2::new(-1., 1.);

            calculate_unbounded_2d_overlap(
                |x| x.clamp(f32::NEG_INFINITY, origin_pos_south_west.x),
                |y| y.clamp(origin_pos_south_west.y, f32::INFINITY),
                candidate_pos,
                candidate_size,
            )
        }
        CompassOctant::NorthWest => {
            // Check for overlap in the NorthWest quadrant with its center at the origin's NW Corner
            let origin_pos_north_west = origin_pos - (origin_size / 2.);

            calculate_unbounded_2d_overlap(
                |x| x.clamp(f32::NEG_INFINITY, origin_pos_north_west.x),
                |y| y.clamp(f32::NEG_INFINITY, origin_pos_north_west.y),
                candidate_pos,
                candidate_size,
            )
        }
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
    let distance = bevy_math::ops::sqrt(dx * dx + dy * dy);

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
/// For details on what "best" means here, refer to [`AutoNavigationConfig`], which configures
/// how candidates are scored.
pub fn find_best_candidate(
    origin: &FocusableArea,
    direction: CompassOctant,
    candidates: &[FocusableArea],
    config: &AutoNavigationConfig,
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
    fn test_calculate_overlap_diagonal() {
        let origin_pos = Vec2::new(100.0, 100.0);
        let origin_size = Vec2::new(50.0, 50.0);

        // Fully overlapping candidate to the south east
        let south_east_pos = Vec2::new(150.0, 150.0);
        let south_east_size = Vec2::new(30.0, 30.0);
        let overlap = calculate_overlap(
            origin_pos,
            origin_size,
            south_east_pos,
            south_east_size,
            CompassOctant::SouthEast,
        );
        assert_eq!(overlap, 1.0); // Full overlap

        // Partially overlapping node to the south east
        let south_east_pos = Vec2::new(120.0, 150.0);
        let partial_overlap = calculate_overlap(
            origin_pos,
            origin_size,
            south_east_pos,
            south_east_size,
            CompassOctant::SouthEast,
        );
        assert!(partial_overlap > 0.0 && partial_overlap < 1.0);

        // No overlap
        let south_east_pos = Vec2::new(110.0, 150.0);
        let no_overlap = calculate_overlap(
            origin_pos,
            origin_size,
            south_east_pos,
            south_east_size,
            CompassOctant::SouthEast,
        );
        assert_eq!(no_overlap, 0.0);
    }
}
