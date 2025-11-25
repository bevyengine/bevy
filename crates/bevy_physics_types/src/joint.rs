//! Joint physics constraints.
//!
//! PhysicsJoint constrains the movement of rigid bodies. A joint can be created
//! between two rigid bodies or between one rigid body and the world. By default,
//! a joint primitive defines a D6 joint where all degrees of freedom are free
//! (three linear and three angular). Note that the default behavior is to disable
//! collision between jointed bodies.
//! 
//! impl SHOULD delete this entity if joint is pointing to one ore more invalid entitites.
//! impl MUST NOT apply forces from invalid joints. 
//! Impl MUST delete broken joints.
//! 
//! if Body0 or Body1 component is missing, this joint is anchored to world space.
use bevy_ecs::entity::Entity;
use bevy_math::prelude::*;

usd_marker! {
    /// Marks this entity as a physics joint constraining rigid bodies.
    PhysicsJoint;
    apiName = "jointEnabled"
    displayName = "Physics Joint"
}

usd_attribute! {
    /// Relationship to first connected body.
    /// If missing, fixed.
    Body0(Entity);
    apiName = "body0"
    displayName = "Body 0"
}

usd_attribute! {
    /// Relationship to second connected body.
    /// If missing, fixed.
    Body1(Entity);
    apiName = "body1"
    displayName = "Body 1"
}

usd_attribute! {
    /// Relative position of the joint frame to body0's frame.
    LocalPos0(Vec3) = vec3(0.0, 0.0, 0.0);
    apiName = "localPos0"
    displayName = "Local Position 0"
}

usd_attribute! {
    /// Relative orientation of the joint frame to body0's frame.
    LocalRot0(Quat) = quat(1.0, 0.0, 0.0, 0.0);
    apiName = "localRot0"
    displayName = "Local Rotation 0"
}

usd_attribute! {
    /// Relative position of the joint frame to body1's frame.
    LocalPos1(Vec3) = vec3(0.0, 0.0, 0.0);
    apiName = "localPos1"
    displayName = "Local Position 1"
}

usd_attribute! {
    /// Relative orientation of the joint frame to body1's frame.
    LocalRot1(Quat) = quat(1.0, 0.0, 0.0, 0.0);
    apiName = "localRot1"
    displayName = "Local Rotation 1"
}


usd_marker! {
    /// Determines if the joint can be included in an Articulation.
    ExcludeFromArticulation;
    apiName = "excludeFromArticulation"
    displayName = "Exclude From Articulation"
}

usd_attribute! {
    /// Joint break force. If set, joint is to break when this force
    /// limit is reached. (Used for linear DOFs.)
    /// Units: mass * distance / second / second
    BreakForce(f32) = f32::INFINITY;
    apiName = "breakForce"
    displayName = "Break Force"
}

usd_attribute! {
    /// Joint break torque. If set, joint is to break when this torque
    /// limit is reached. (Used for angular DOFs.)
    /// Units: mass * distance * distance / second / second
    BreakTorque(f32) = f32::INFINITY;
    apiName = "breakTorque"
    displayName = "Break Torque"
}
