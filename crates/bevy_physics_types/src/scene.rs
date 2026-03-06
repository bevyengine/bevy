//! Physics scene configuration for simulation-level properties.
//!
//! The [`PhysicsSimulation`] defines sim-level physics simulation properties.
//! Multiple independent physics simulations can be described within a single
//! scene by creating multiple simulation entities.
//!
//! ## Gravity Configuration
//!
//! The scene controls gravity via separate direction and magnitude components:
//! - [`GravityDirection`]: A normalized vector in simulation world space. When not
//!   specified (or zero), the negative scene upAxis is used as default.
//! - [`GravityMagnitude`]: The acceleration magnitude. A negative sentinel value
//!   (-inf or similar) requests earth-equivalent gravity (9.81 m/s²) regardless
//!   of the scene's `metersPerUnit` scaling.
//!
//! ## Multi-Scene Support
//!
//! Bodies are assigned to specific scenes using the [`SimulationOwner`] relationship.
//! If there is only one unique sim, an explicit relationship is unnecessary—bodies
//! are assumed to be associated with the singleton sim.
//!
//! **Note**: A single body cannot belong to multiple scenes as this would create
//! data races and conflicting simulation states.
//!
//! ## Units
//!
//! Gravity magnitude uses units of `distance/second²`. The actual physical value
//! depends on the scene's `metersPerUnit` metadata. For example, if `metersPerUnit = 0.01`
//! (centimeters), then earth gravity would be approximately 981 in scene units.

use crate::types::float;
use bevy_ecs::entity::EntityHashSet;
use bevy_math::Dir3;

make_attribute! {
    /// Gravity direction vector in simulation world space.
    ///
    /// When not set, implementations should use the negative scene upAxis as the
    /// gravity direction (typically -Y or -Z depending on scene configuration).
    ///
    /// Unitless (normalized direction).
    GravityDirection(bevy_math::Dir3) = Dir3::NEG_Y;
    apiName = "gravityDirection"
    displayName = "Gravity Direction"
}

make_attribute! {
    /// Gravity acceleration magnitude in simulation world space.
    ///
    /// A negative sentinel value (the default -9.81 or -inf) is a request to use
    /// a value equivalent to earth gravity (9.81 m/s²) regardless of the
    /// `metersPerUnit` scaling used by this scene. The implementation should
    /// convert this to appropriate scene units.
    ///
    /// Units: distance/second².
    GravityMagnitude(float) = -9.81;
    apiName = "gravityMagnitude"
    displayName = "Gravity Magnitude"
}

make_collection! {
    /// Relationship to the PhysicsSimulation that will simulate this entity.
    ///
    /// This component establishes which physics sim owns and simulates the
    /// associated rigid body or collider. When only one PhysicsSimulation exists
    /// in the scene, this relationship may be omitted and bodies will
    /// automatically belong to that singleton sim.
    ///
    /// For static colliders not under a RigidBody, this relationship determines
    /// which sim handles their collision detection.
    SimulationOwner->PhysicsSimulation(EntityHashSet);
    apiName = "simulationOwner"
    displayName = "Simulation Owner"
}
