use bevy_ecs::{entity::EntityHashSet, prelude::Component};
use bevy_math::Vec3;

/// A PhysicsScene
/// owns the backend
/// probably only has one
#[derive(Component, Debug, Default)]
#[relationship_target(relationship = crate::rigid_body::SimulationOwner, linked_spawn)]
pub struct PhysicsScene(EntityHashSet);

/// Gravity vector in simulation world space.
/// equivalent to earth gravity regardless of the metersPerUnit scaling used by this scene. Units: distance/second/second.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Gravity(pub Vec3);

/// Backend-specific hint for iter counts.
/// in general more iterations is more accurate but slower
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolverIterationsHint(u8);