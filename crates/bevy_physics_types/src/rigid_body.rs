use bevy_ecs::prelude::{Component, Entity};
use bevy_math::{Quat, Vec3};

use crate::scene::PhysicsScene;

/// root of physics rigidbody tree
/// USD `PhysicsRigidBodyAPI`:
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RigidBody;

/// linear velocity in the same space as the node's xform.
/// Units: distance/second.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct LinearVelocity(pub Vec3);

impl Default for LinearVelocity {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Angular velocity in the same space as the node's xform.
/// Units: degrees/second.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct AngularVelocity(pub Vec3);

impl Default for AngularVelocity {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// promise that the item won't move
/// allows for performance optimizations
/// incompatable with dynmaic
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Static;

/// USD `physics:rigidBodyEnabled`
/// position driven by physics engine 
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dynamic;

/// causes this rigdbody to be manipulated by external force
/// NOT normal kinematic
/// if you want kinematic, remove this.
/// USD `physics:kinematicEnabled`:
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalForce;

/// PhysicsScene that will simulate this body.
/// USD `physics:simulationOwner`
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
#[relationship(relationship_target = PhysicsScene)]
pub struct SimulationOwner(pub Entity);

/// USD `physics:startsAsleep`
/// causes the body to be asleep when the simulation starts.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartsAsleep;

/// Runtime sleeping feedback.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BodySleeping(pub bool);

/// can this be auto slept
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AutoSleep;
