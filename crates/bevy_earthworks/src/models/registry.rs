//! Machine model registry and asset management.

use bevy_asset::{AssetServer, Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_gltf::Gltf;
use bevy_log::info;
use bevy_reflect::Reflect;
use bevy_scene::Scene;
use std::collections::HashMap;

use crate::machines::MachineType;

/// State of model loading for a specific machine type.
#[derive(Clone, Debug, Default, PartialEq, Eq, Reflect)]
pub enum ModelLoadState {
    /// Model has not been requested yet.
    #[default]
    NotLoaded,
    /// Model is currently loading.
    Loading,
    /// Model loaded successfully.
    Loaded,
    /// Model failed to load, will use procedural fallback.
    Failed,
    /// No model configured, using procedural geometry.
    Procedural,
}

/// Configuration for a machine model.
#[derive(Clone, Debug)]
pub struct MachineModelConfig {
    /// Path to the GLTF file (relative to assets folder).
    pub gltf_path: Option<String>,
    /// Name of the scene within the GLTF to use (if multiple scenes).
    pub scene_name: Option<String>,
    /// Scale factor to apply to the model.
    pub scale: f32,
    /// Y offset to position model correctly on ground.
    pub y_offset: f32,
}

impl Default for MachineModelConfig {
    fn default() -> Self {
        Self {
            gltf_path: None,
            scene_name: None,
            scale: 1.0,
            y_offset: 0.0,
        }
    }
}

/// Registry of machine models and their configurations.
#[derive(Resource, Default)]
pub struct MachineModelRegistry {
    /// Model configurations per machine type.
    configs: HashMap<MachineType, MachineModelConfig>,
    /// Load state per machine type.
    load_states: HashMap<MachineType, ModelLoadState>,
}

impl MachineModelRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a model for a machine type.
    pub fn register(&mut self, machine_type: MachineType, config: MachineModelConfig) {
        self.configs.insert(machine_type, config);
        self.load_states.insert(machine_type, ModelLoadState::NotLoaded);
    }

    /// Gets the model configuration for a machine type.
    pub fn get_config(&self, machine_type: MachineType) -> Option<&MachineModelConfig> {
        self.configs.get(&machine_type)
    }

    /// Gets the load state for a machine type.
    pub fn get_load_state(&self, machine_type: MachineType) -> ModelLoadState {
        self.load_states
            .get(&machine_type)
            .cloned()
            .unwrap_or(ModelLoadState::Procedural)
    }

    /// Sets the load state for a machine type.
    pub fn set_load_state(&mut self, machine_type: MachineType, state: ModelLoadState) {
        self.load_states.insert(machine_type, state);
    }

    /// Returns true if all registered models are loaded (or failed).
    pub fn all_loaded(&self) -> bool {
        self.load_states.values().all(|state| {
            matches!(
                state,
                ModelLoadState::Loaded | ModelLoadState::Failed | ModelLoadState::Procedural
            )
        })
    }

    /// Returns true if a model is available for the given machine type.
    pub fn has_model(&self, machine_type: MachineType) -> bool {
        self.get_load_state(machine_type) == ModelLoadState::Loaded
    }
}

/// Holds loaded model asset handles.
#[derive(Resource, Default)]
pub struct ModelAssets {
    /// GLTF handles per machine type.
    pub gltf_handles: HashMap<MachineType, Handle<Gltf>>,
    /// Scene handles extracted from GLTFs.
    pub scene_handles: HashMap<MachineType, Handle<Scene>>,
}

impl ModelAssets {
    /// Gets the scene handle for a machine type.
    pub fn get_scene(&self, machine_type: MachineType) -> Option<Handle<Scene>> {
        self.scene_handles.get(&machine_type).cloned()
    }
}

/// System to set up the model registry with default paths.
pub fn setup_model_registry(mut registry: ResMut<MachineModelRegistry>) {
    // Register default model paths
    // These will be loaded if the files exist, otherwise procedural geometry is used

    registry.register(
        MachineType::Dozer,
        MachineModelConfig {
            gltf_path: Some("models/bulldozer.glb".to_string()),
            scene_name: None,
            scale: 1.0,
            y_offset: 0.0,
        },
    );

    registry.register(
        MachineType::Excavator,
        MachineModelConfig {
            gltf_path: Some("models/excavator.glb".to_string()),
            scene_name: None,
            scale: 1.0,
            y_offset: 0.0,
        },
    );

    registry.register(
        MachineType::Loader,
        MachineModelConfig {
            gltf_path: Some("models/loader.glb".to_string()),
            scene_name: None,
            scale: 1.0,
            y_offset: 0.0,
        },
    );

    registry.register(
        MachineType::DumpTruck,
        MachineModelConfig {
            gltf_path: Some("models/dump_truck.glb".to_string()),
            scene_name: None,
            scale: 1.0,
            y_offset: 0.0,
        },
    );

    info!("Machine model registry initialized with {} entries", 4);
}

/// System to check model load state and trigger loading.
pub fn check_model_load_state(
    asset_server: Res<AssetServer>,
    gltf_assets: Res<Assets<Gltf>>,
    mut registry: ResMut<MachineModelRegistry>,
    mut model_assets: ResMut<ModelAssets>,
) {
    for machine_type in [
        MachineType::Dozer,
        MachineType::Excavator,
        MachineType::Loader,
        MachineType::DumpTruck,
    ] {
        let current_state = registry.get_load_state(machine_type);

        match current_state {
            ModelLoadState::NotLoaded => {
                // Try to load the model
                if let Some(config) = registry.get_config(machine_type) {
                    if let Some(ref path) = config.gltf_path {
                        let handle: Handle<Gltf> = asset_server.load(path.clone());
                        model_assets.gltf_handles.insert(machine_type, handle);
                        registry.set_load_state(machine_type, ModelLoadState::Loading);
                    } else {
                        registry.set_load_state(machine_type, ModelLoadState::Procedural);
                    }
                } else {
                    registry.set_load_state(machine_type, ModelLoadState::Procedural);
                }
            }
            ModelLoadState::Loading => {
                // Check if the GLTF is loaded
                if let Some(handle) = model_assets.gltf_handles.get(&machine_type) {
                    match asset_server.get_load_state(handle.id()) {
                        Some(bevy_asset::LoadState::Loaded) => {
                            // Extract the scene from the GLTF
                            if let Some(gltf) = gltf_assets.get(handle) {
                                // Use default scene or first named scene
                                let scene_handle = gltf.default_scene.clone()
                                    .or_else(|| gltf.scenes.first().cloned());

                                if let Some(scene) = scene_handle {
                                    model_assets.scene_handles.insert(machine_type, scene);
                                    registry.set_load_state(machine_type, ModelLoadState::Loaded);
                                    info!("Loaded model for {:?}", machine_type);
                                } else {
                                    registry.set_load_state(machine_type, ModelLoadState::Failed);
                                    info!("No scene found in GLTF for {:?}, using procedural", machine_type);
                                }
                            }
                        }
                        Some(bevy_asset::LoadState::Failed(_)) => {
                            registry.set_load_state(machine_type, ModelLoadState::Failed);
                            info!("Failed to load model for {:?}, using procedural fallback", machine_type);
                        }
                        _ => {
                            // Still loading, do nothing
                        }
                    }
                }
            }
            _ => {
                // Already loaded, failed, or procedural - nothing to do
            }
        }
    }
}
