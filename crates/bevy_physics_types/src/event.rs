use bevy_ecs::prelude::Entity;
use bevy_ecs_macros::EntityEvent;
use bevy_math::{Dir3, Vec3};

/// Collision start event emitted when two bodies begin to collide.
#[derive(EntityEvent, Debug, Clone, Copy, PartialEq)]
pub struct CollisionStart {
    #[event_target]
	/// First colliding body (authoring order or engine dependent)
	pub collider: Entity,
	/// what colliding with
	pub other: Entity,
	/// world-space contact point where collision began
	pub contact_point: Vec3,
	/// world-space contact normal pointing away from `self` toward `other`.
	pub contact_normal: Dir3,
	/// scalar impulse applied during the collision event
	pub impulse: f32,
	/// relative linear velocity vector (other - self) at contact in world space
	pub relative_velocity: Vec3,
}
