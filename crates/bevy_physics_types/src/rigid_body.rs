//! Rigid body physics properties.
//!
//! The PhysicsRigidBodyAPI applies physics body attributes to any UsdGeomXformable
//! prim and marks it to be driven by a simulation. If a simulation is running,
//! it will update the prim's pose. All prims in the hierarchy below are moved
//! rigidly with the body, except descendants with their own PhysicsRigidBodyAPI
//! which move independently. This API supports kinematic bodies (moved through
//! animated poses), sleeping states, and velocity tracking for both linear and
//! angular motion.
use bevy_math::prelude::*;

usd_marker! {
    /// marks the root of a rigid body
    RigidBody;
    apiName = "rigidBodyApi"
    displayName = "Rigid Body"
}

usd_marker! {
    /// causes the entity to react to external forces
    Dynamic;
    apiName = "rigidBodyEnabled"
    displayName = "Rigid Body Enabled"
}

usd_marker! {
    /// The physics engine won't move this entity.
    /// Determines whether the body is kinematic or not. A kinematic
    /// body is a body that is moved through animated poses or through
    /// user defined poses. The simulation derives velocities for the
    /// kinematic body based on the external motion. When a continuous motion
    /// is not desired, this kinematic flag should be set to false.
    Kinematic;
    apiName = "kinematicEnabled"
    displayName = "Kinematic Enabled"
}

usd_marker! {
    /// Determines if the body is asleep when the simulation starts.
    StartsAsleep;
    apiName = "startsAsleep"
    displayName = "Starts as Asleep"
}

usd_attribute! {
    /// Linear velocity in the same space as the node's xform.
    /// Units: distance/second.
    Velocity(Vec3) = vec3(0.0, 0.0, 0.0);
    apiName = "velocity"
    displayName = "Linear Velocity"
}

usd_attribute! {
    /// Angular velocity in the same space as the node's xform.
    /// Units: degrees/second.
    AngularVelocity(Vec3) = vec3(0.0, 0.0, 0.0);
    apiName = "angularVelocity"
    displayName = "Angular Velocity"
}
