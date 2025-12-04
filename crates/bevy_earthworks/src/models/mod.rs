//! Machine model loading and management.
//!
//! This module handles loading GLTF models for machines and provides
//! fallback procedural geometry when models aren't available.

mod registry;
mod spawner;
mod procedural;

pub use registry::{MachineModelRegistry, ModelAssets, ModelLoadState, MachineModelConfig};
pub use spawner::{spawn_machine_with_model, MachineSpawner, MachineEntity};
pub use procedural::{
    spawn_procedural_dozer, spawn_procedural_excavator,
    spawn_procedural_loader, spawn_procedural_dump_truck,
    ProceduralMachineVisual,
};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Plugin for machine model loading and management.
pub struct ModelsPlugin;

impl Plugin for ModelsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MachineModelRegistry>()
            .init_resource::<ModelAssets>()
            .add_systems(Startup, registry::setup_model_registry)
            .add_systems(Update, registry::check_model_load_state);
    }
}
