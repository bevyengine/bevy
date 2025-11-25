//! General physics simulation properties, required for simulation.
//!
//! The PhysicsScene class defines stage-level physics simulation properties.
//! This scene controls gravity direction and magnitude that affects all physics
//! bodies within the simulation. Gravity direction is a normalized vector in
//! simulation world space, and a zero vector requests the use of the negative
//! upAxis. Gravity magnitude can be a negative value to request earth-equivalent
//! gravity regardless of scene scaling.

use bevy_ecs::entity::EntityHashSet;
use bevy_math::Dir3;

usd_attribute! {
    /// Gravity direction vector in simulation world space.
    /// Missing is a request to use the negative upAxis.
    /// Unitless.
    GravityDirection(bevy_math::Dir3) = Dir3::NEG_Y;
    apiName = "gravityDirection"
    displayName = "Gravity Direction"
}

usd_attribute! {
    /// Gravity acceleration magnitude in simulation world space.
    /// A negative value is a request to use a value equivalent to earth
    /// gravity regardless of the metersPerUnit scaling used by this scene.
    /// Units: distance/second/second.
    GravityMagnitude(f32) = -9.81;
    apiName = "gravityMagnitude"
    displayName = "Gravity Magnitude"
}

usd_collection! {
    /// PhysicsScene that will simulate this
    /// This componenet MAY be added if missing if other physics componenets are on entity.
    SimulationOwner->PhysicsSimulation(EntityHashSet);
    apiName = "simulationOwner"
    displayName = "Simulation Owner"
}
