//! Distance joint type.
//!
//! A [`DistanceJoint`] constrains the distance between two anchor points on
//! connected bodies. Unlike other joints that constrain specific axes, this
//! joint constrains the scalar distance between connection points.
//!
//! ## Behavior
//!
//! The joint attachment points (defined by [`LocalPos0`](crate::joint::LocalPos0)
//! and [`LocalPos1`](crate::joint::LocalPos1)) are constrained to maintain a
//! distance within the specified range.
//!
//! - **Min distance**: Bodies are pushed apart if too close
//! - **Max distance**: Bodies are pulled together if too far
//!
//! ## Limit Independence
//!
//! The minimum and maximum limits can be enabled independently:
//! - Set `min_distance` negative to disable minimum constraint
//! - Set `max_distance` negative to disable maximum constraint
//! - Both negative = no distance constraint (joint has no effect)
//!
//! ## Example Uses
//!
//! - Rope or chain links (max distance only)
//! - Rigid rods (min = max distance)
//! - Springy connections with slack
//! - Keeping objects within range of each other

use crate::types::float;
use bevy_ecs::component::Component;

/// A distance joint constraining the distance between attachment points.
///
/// This joint type maintains a minimum and/or maximum distance between
/// the joint anchor points on each body.
#[derive(Component)]
pub struct DistanceJoint {
    /// Minimum allowed distance between attachment points.
    ///
    /// If the distance falls below this, the joint applies a repulsive
    /// force/constraint to push the bodies apart.
    ///
    /// A negative value disables the minimum distance constraint.
    ///
    /// Units: distance.
    pub min_distance: float,

    /// Maximum allowed distance between attachment points.
    ///
    /// If the distance exceeds this, the joint applies an attractive
    /// force/constraint to pull the bodies together.
    ///
    /// A negative value disables the maximum distance constraint.
    ///
    /// Units: distance.
    pub max_distance: float,
}

impl Default for DistanceJoint {
    fn default() -> Self {
        Self {
            min_distance: -1.0,
            max_distance: -1.0,
        }
    }
}
