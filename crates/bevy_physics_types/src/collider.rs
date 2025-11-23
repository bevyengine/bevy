//! USD `PhysicsCollisionAPI`
use bevy_ecs::prelude::Component;
use bevy_math::{
    Quat, Vec3,
    primitives::{Capsule3d, Cuboid, Cylinder, InfinitePlane3d, Plane3d, Sphere},
};

/// If any prim in the parent hierarchy has the RigidBodyAPI applied, the collider is considered part of the closest ancestor body.
/// If there is no body in the parent hierarchy, this collider is considered to be static.
/// Geometric description for the collider
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum ColliderShape {
    Sphere(Sphere),
    Capsule(Capsule3d),
    Cuboid(Cuboid),
    Cylinder(Cylinder),
    Plane(Plane3d),
    HalfSpace(InfinitePlane3d),
    Shared(Arc<Box<dyn ColliderShape>>),
}

/// raycasted movement
/// can't collide with other points
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PointCollider;

/// super type for implemeting custom colliders
trait ColliderShape {
    // TODO
}


mod shared {
    use bevy_ecs::prelude::Component;
}
