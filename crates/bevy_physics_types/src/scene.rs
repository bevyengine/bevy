//! Physics scene configuration for simulation-level properties.
//!
//! The [`PhysicsSimulation`] represents a USD PhysicsScene which defines stage-level
//! physics simulation properties. Multiple independent physics simulations can be
//! described within a single USD stage by creating multiple scenes.
//!
//! ## Gravity Configuration
//!
//! The scene controls gravity via separate direction and magnitude attributes:
//! - [`GravityDirection`]: A normalized vector in simulation world space. When not
//!   specified (or zero), the negative stage upAxis is used as default.
//! - [`GravityMagnitude`]: The acceleration magnitude. A negative sentinel value
//!   (-inf or similar) requests earth-equivalent gravity (9.81 m/s²) regardless
//!   of the stage's `metersPerUnit` scaling.
//!
//! ## Multi-Scene Support
//!
//! Bodies are assigned to specific scenes using the [`SimulationOwner`] relationship.
//! If there is only one unique scene in the stage, an explicit relationship is
//! unnecessary—bodies are assumed to be associated with the singleton scene.
//!
//! **Note**: A single body cannot belong to multiple scenes as this would create
//! data races and conflicting simulation states.
//!
//! ## Units
//!
//! Gravity magnitude uses units of `distance/second²`. The actual physical value
//! depends on the stage's `metersPerUnit` metadata. For example, if `metersPerUnit = 0.01`
//! (centimeters), then earth gravity would be approximately 981 in stage units.

use bevy_ecs::entity::EntityHashSet;
use bevy_math::Dir3;

usd_attribute! {
    /// Gravity direction vector in simulation world space.
    ///
    /// This should be a normalized direction vector. When set to the default
    /// or a zero vector, implementations should use the negative stage upAxis
    /// as the gravity direction (typically -Y or -Z depending on stage configuration).
    ///
    /// Unitless (normalized direction).
    GravityDirection(bevy_math::Dir3) = Dir3::NEG_Y;
    apiName = "gravityDirection"
    displayName = "Gravity Direction"
}

usd_attribute! {
    /// Gravity acceleration magnitude in simulation world space.
    ///
    /// A negative sentinel value (the default -9.81 or -inf) is a request to use
    /// a value equivalent to earth gravity (9.81 m/s²) regardless of the
    /// `metersPerUnit` scaling used by this stage. The implementation should
    /// convert this to appropriate stage units.
    ///
    /// Units: distance/second².
    GravityMagnitude(f32) = -9.81;
    apiName = "gravityMagnitude"
    displayName = "Gravity Magnitude"
}

usd_collection! {
    /// Relationship to the PhysicsScene that will simulate this object.
    ///
    /// This component establishes which physics scene owns and simulates the
    /// associated rigid body or collider. When only one PhysicsScene exists
    /// in the stage, this relationship may be omitted and bodies will
    /// automatically belong to that singleton scene.
    ///
    /// For static colliders not under a RigidBody, this relationship determines
    /// which scene handles their collision detection.
    SimulationOwner->PhysicsSimulation(EntityHashSet);
    apiName = "simulationOwner"
    displayName = "Simulation Owner"
}
