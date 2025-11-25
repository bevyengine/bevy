//! Fixed joint type with all degrees of freedom removed.
//!
//! PhysicsFixedJoint defines a predefined fixed joint type where all degrees of freedom
//! are removed. The two connected bodies are rigidly connected.

use bevy_ecs_macros::Component;

/// Marks this entity as a fixed joint with all degrees of freedom removed.
#[derive(Default, Component)]
pub struct FixedJoint;
