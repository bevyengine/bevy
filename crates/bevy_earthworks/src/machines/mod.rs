//! Construction machine system.
//!
//! This module provides:
//! - [`Machine`] - Core machine component
//! - [`MachineType`] - Types of construction machines (Excavator, Dozer, Loader)
//! - [`WorkEnvelope`] - Defines the operational reach of a machine
//! - [`MachineActivity`] - Current activity state machine
//! - Animation and interpolation systems

mod animation;
mod catalog;
mod components;
mod direct_control;
mod gizmos;

pub use animation::animation_system;
pub use catalog::MachineCatalog;
pub use components::{
    Machine, MachineActivity, MachineBundle, MachineType, Mobility, WorkEnvelope,
};
pub use direct_control::{
    BladeState, BladeVisual, ControlResponse, DirectControlPlugin, DozerPushState, PlayerControlled,
};
pub use gizmos::draw_work_envelopes;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Plugin for machine systems.
pub struct MachinesPlugin;

impl Plugin for MachinesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MachineCatalog>()
            .add_message::<MachineEvent>()
            .add_systems(Update, animation_system);
    }
}

/// Events emitted by machines.
#[derive(Message, Clone, Debug)]
pub enum MachineEvent {
    /// Machine started moving to a new position.
    StartedMoving {
        /// The machine entity.
        entity: Entity,
        /// Target position.
        target: bevy_math::Vec3,
    },
    /// Machine reached its destination.
    ReachedDestination {
        /// The machine entity.
        entity: Entity,
    },
    /// Machine started an excavation operation.
    StartedExcavating {
        /// The machine entity.
        entity: Entity,
    },
    /// Machine completed an excavation operation.
    CompletedExcavating {
        /// The machine entity.
        entity: Entity,
        /// Volume excavated.
        volume: f32,
    },
    /// Machine started a dump operation.
    StartedDumping {
        /// The machine entity.
        entity: Entity,
    },
    /// Machine completed a dump operation.
    CompletedDumping {
        /// The machine entity.
        entity: Entity,
        /// Volume dumped.
        volume: f32,
    },
}
