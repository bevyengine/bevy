//! Rigid body physics properties.
//!
//! The [`RigidBody`] marker applies physics body attributes to any transformable
//! entity and marks it to be driven by a physics simulation. If a simulation is
//! running, it will update the entity's pose (position and orientation).
//!
//! ## Hierarchy Behavior
//!
//! All entities in the hierarchy below a [`RigidBody`] are assumed to be part of
//! that rigid body and move rigidly along with it. This is consistent with the
//! common behavior expected during hand-animation of a subtree.
//!
//! **Exception**: When a descendant entity has its own [`RigidBody`], it creates
//! a separate rigid body and forms the root of its own subtree. For all physics
//! purposes, this entity and all entities below it are not considered parts of
//! bodies higher up in the hierarchy.
//!
//! ## Body Types
//!
//! - **Dynamic** ([`Dynamic`]): Bodies that respond to forces and collisions.
//!   The simulation computes their motion based on physics.
//! - **Kinematic** ([`Kinematic`]): Bodies moved through animated poses or user
//!   input. The simulation reads (but does not write) their transforms and derives
//!   velocities from the external motion. Kinematic bodies still push dynamic bodies
//!   and pull on joints, imparting velocity during collisions.
//! - **Static**: Entities with [`CollisionEnabled`](crate::collision::CollisionEnabled)
//!   but no [`RigidBody`] are treated as static colliders with infinite mass and
//!   zero velocity.
//!
//! ## Sleep State
//!
//! Large terrestrial simulations often have objects that come to rest. Physics
//! engines typically "sleep" such bodies to improve performance, ceasing updates
//! until equilibrium is disturbed. The [`StartsAsleep`] marker allows bodies to
//! begin simulation in a sleeping state.
//!
//! ## Velocity
//!
//! Velocities ([`Velocity`] and [`AngularVelocity`]) are specified in the entity's
//! local space. Angular velocity uses degrees per second.

use crate::types::vector3f;

make_marker! {
    /// Marks the root of a rigid body hierarchy.
    ///
    /// When applied to an entity, this entity becomes the root of a rigid body
    /// and all descendant entities (without their own RigidBody) move as a single
    /// rigid unit. The physics simulation will update this entity's transform.
    RigidBody;
    apiName = "rigidBodyApi"
    displayName = "Rigid Body"
}

make_marker! {
    /// Enables dynamic simulation for the rigid body.
    ///
    /// When present, the body responds to external forces, gravity, and collisions.
    /// The physics simulation computes and updates the body's motion.
    // When absent the body behaves as a static or kinematic body,
    // depending on other properties.
    Dynamic;
    apiName = "rigidBodyEnabled"
    displayName = "Rigid Body Enabled"
}

make_marker! {
    /// Marks a body as kinematic (animation-driven).
    ///
    /// A kinematic body is moved through animated poses or user-defined poses
    /// rather than physics simulation. The simulation derives velocities for
    /// the kinematic body based on the external motion, and this velocity will
    /// be imparted to dynamic bodies during collisions.
    ///
    /// Unlike a static collider, kinematic bodies:
    /// - Have continuous velocity inferred from their motion
    /// - Can push dynamic bodies with realistic momentum transfer
    /// - Can drive joints they're connected to
    ///
    /// When continuous motion is not desired, this should be disabled.
    Kinematic;
    apiName = "kinematicEnabled"
    displayName = "Kinematic Enabled"
}

make_marker! {
    /// Indicates the body should start simulation in a sleeping state.
    ///
    /// Sleeping is a performance optimization where bodies at rest cease being
    /// updated until disturbed. This marker allows bodies to begin simulation
    /// already asleep, useful for scenes with many resting objects.
    StartsAsleep;
    apiName = "startsAsleep"
    displayName = "Starts as Asleep"
}

make_attribute! {
    /// Linear velocity in the body's local space.
    ///
    /// This velocity is specified relative to the entity's local coordinate frame.
    ///
    /// Units: distance/second.
    Velocity(vector3f) = vector3f::ZERO;
    apiName = "velocity"
    displayName = "Linear Velocity"
}

make_attribute! {
    /// Angular velocity in the body's local space.
    ///
    /// This velocity is specified relative to the entity's local coordinate frame.
    ///
    /// Units: radians/second.
    AngularVelocity(vector3f) = vector3f::ZERO;
    apiName = "angularVelocity"
    displayName = "Angular Velocity"
}
