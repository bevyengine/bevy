//! Joint physics constraints between rigid bodies.
//!
//! Joints are constraints that create fixed spatial relationships between rigid
//! bodies. They represent attachments like a drawer to a cabinet, a wheel to a car,
//! or robot arm links to each other.
//!
//! ## Joint Types
//!
//! - **Generic D6 Joint** ([`PhysicsJoint`]): All six degrees of freedom configurable
//!   via [`LimitAPI`](crate::limit) and [`DriveAPI`](crate::drive). Default is fully free.
//! - **[`FixedJoint`](crate::joint_fixed::FixedJoint)**: Locks all DOFs.
//! - **[`RevoluteJoint`](crate::joint_revolute::RevoluteJoint)**: Rotation around one axis.
//! - **[`PrismaticJoint`](crate::joint_prismatic::PrismaticJoint)**: Translation along one axis.
//! - **[`SphericalJoint`](crate::joint_spherical::SphericalJoint)**: Ball-and-socket, removes linear DOFs.
//! - **[`DistanceJoint`](crate::joint_distance::DistanceJoint)**: Min/max distance constraint.
//!
//! ## Joint Reference Frames
//!
//! Joints are defined by **two distinct frames**, one relative to each connected body.
//! This is necessary because:
//! - The joint allows relative motion (like a car suspension moving up/down)
//! - Approximate simulations may allow slight separation under high forces
//!
//! The joint frames should generally align in world space along constrained DOFs.
//! [`LocalPos0`]/[`LocalRot0`] define the frame relative to body0, and
//! [`LocalPos1`]/[`LocalRot1`] define the frame relative to body1.
//!
//! **Note**: Joint space is translation and orientation only—scaling is not supported.
//! Real-world objects cannot scale arbitrarily, and simulations don't support it.
//!
//! ## Connected Bodies
//!
//! [`Body0`] and [`Body1`] relationships define the connected bodies. If either
//! relationship is undefined, the joint attaches to the **static world frame**.
//! At least one body should have [`RigidBody`](crate::rigid_body::RigidBody) for
//! meaningful simulation.
//!
//! ## Collision Filtering
//!
//! By **default, collisions between jointed bodies are disabled** to prevent
//! collision shapes from interfering. This can be overridden by applying
//! [`CollisionEnabled`](crate::collision::CollisionEnabled) to the joint.
//! No filtering occurs if either body relationship is undefined.
//!
//! ## Breaking Joints
//!
//! Joints can break when sufficient force ([`BreakForce`]) or torque ([`BreakTorque`])
//! is applied. This models real-world behavior like a door being ripped off its hinges.
//! Set to infinity (default) for unbreakable joints.
//!
//! ## Implementation Notes
//!
//! - Implementations SHOULD delete joint entities pointing to invalid bodies
//! - Implementations MUST NOT apply forces from invalid joints
//! - Implementations MUST delete broken joints

use crate::types::{float, point3f, quatf};
use bevy_ecs::entity::Entity;

make_marker! {
    /// Marks this entity as a physics joint.
    ///
    /// The base joint type represents a D6 joint with all degrees of freedom
    /// free by default. Use [`LimitAPI`](crate::limit) components to restrict
    /// motion and [`DriveAPI`](crate::drive) components to add actuation.
    ///
    /// For common joint configurations, use the specialized joint types instead.
    PhysicsJoint;
    apiName = "jointEnabled"
    displayName = "Physics Joint"
}

make_attribute! {
    /// Relationship to the first connected body.
    ///
    /// This can reference any transformable entity. The actual rigid body is
    /// found by searching up the hierarchy for [`RigidBody`](crate::rigid_body::RigidBody).
    ///
    /// If not specified, the joint is anchored to the static world frame.
    Body0(Entity);
    apiName = "body0"
    displayName = "Body 0"
}

make_attribute! {
    /// Relationship to the second connected body.
    ///
    /// This can reference any transformable entity. The actual rigid body is
    /// found by searching up the hierarchy for [`RigidBody`](crate::rigid_body::RigidBody).
    ///
    /// If not specified, the joint is anchored to the static world frame.
    Body1(Entity);
    apiName = "body1"
    displayName = "Body 1"
}

make_attribute! {
    /// Position of the joint frame relative to body0.
    ///
    /// Defines where the joint attaches to body0 in body0's local space.
    /// This position is in body0's local coordinate frame, which may be
    /// scaled—the joint position will appear in the correct location
    /// regardless of body scaling.
    ///
    /// Units: distance.
    LocalPos0(point3f) = point3f::ZERO;
    apiName = "localPos0"
    displayName = "Local Position 0"
}

make_attribute! {
    /// Orientation of the joint frame relative to body0.
    ///
    /// Defines the joint's rotational alignment relative to body0.
    /// The identity quaternion (1, 0, 0, 0) means the joint frame is
    /// aligned with body0's local axes.
    ///
    /// Unitless (quaternion).
    LocalRot0(quatf) = quatf::IDENTITY;
    apiName = "localRot0"
    displayName = "Local Rotation 0"
}

make_attribute! {
    /// Position of the joint frame relative to body1.
    ///
    /// Defines where the joint attaches to body1 in body1's local space.
    ///
    /// Units: distance.
    LocalPos1(point3f) = point3f::ZERO;
    apiName = "localPos1"
    displayName = "Local Position 1"
}

make_attribute! {
    /// Orientation of the joint frame relative to body1.
    ///
    /// Defines the joint's rotational alignment relative to body1.
    ///
    /// Unitless (quaternion).
    LocalRot1(quatf) = quatf::IDENTITY;
    apiName = "localRot1"
    displayName = "Local Rotation 1"
}

make_marker! {
    /// Excludes this joint from articulation reduced-coordinate solving.
    ///
    /// When a joint would create a loop in an articulation tree, it must
    /// remain a maximal-coordinate joint. Use this marker to explicitly
    /// exclude joints from articulation processing.
    ///
    /// This is required when articulation topology would create closed loops,
    /// which many reduced-coordinate solvers don't support.
    ExcludeFromArticulation;
    apiName = "excludeFromArticulation"
    displayName = "Exclude From Articulation"
}

make_attribute! {
    /// Force threshold at which the joint breaks.
    ///
    /// When the constraint force exceeds this value, the joint is destroyed.
    /// This applies to linear degrees of freedom. Use `f32::INFINITY`
    /// (the default) for an unbreakable joint.
    ///
    /// Units: mass × distance / second².
    BreakForce(float) = float::INFINITY;
    apiName = "breakForce"
    displayName = "Break Force"
}

make_attribute! {
    /// Torque threshold at which the joint breaks.
    ///
    /// When the constraint torque exceeds this value, the joint is destroyed.
    /// This applies to angular degrees of freedom. Use `f32::INFINITY`
    /// (the default) for an unbreakable joint.
    ///
    /// Units: mass × distance² / second².
    BreakTorque(float) = float::INFINITY;
    apiName = "breakTorque"
    displayName = "Break Torque"
}
