//! Fixed (weld) joint type.
//!
//! A [`FixedJoint`] removes all degrees of freedom between two bodies,
//! effectively welding them together. The bodies maintain their relative
//! position and orientation as defined by the joint frames.
//!
//! ## Behavior
//!
//! - **Allowed motion**: None
//! - **Restricted motion**: All translation and rotation
//!
//! ## Use Cases
//!
//! Fixed joints are useful for:
//! - Temporarily connecting bodies that may later be separated
//! - Creating breakable connections (via [`BreakForce`](crate::joint::BreakForce))
//! - Connecting static geometry to dynamic bodies
//! - Anchoring articulation roots to the world
//!
//! ## vs Parent-Child Hierarchy
//!
//! While parent-child transform relationships also create rigid connections,
//! fixed joints differ in that:
//! - They can be broken (via break force/torque)
//! - They can be enabled/disabled at runtime
//! - They connect bodies in different subtrees
//! - They create explicit physics constraints
//!
//! ## Example Uses
//!
//! - Robot base bolted to floor
//! - Glued objects that can break under stress
//! - Temporary assembly connections
//! - Mounting points for articulated mechanisms

use bevy_ecs::component::Component;

/// A fixed joint that welds two bodies together.
///
/// This joint type removes all degrees of freedom, making the connected
/// bodies move as a single rigid unit. Unlike a parent-child hierarchy,
/// this connection can be broken via [`BreakForce`](crate::joint::BreakForce)
/// and [`BreakTorque`](crate::joint::BreakTorque).
#[derive(Default, Component)]
pub struct FixedJoint;
