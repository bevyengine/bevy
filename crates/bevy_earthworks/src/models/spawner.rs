//! Machine spawning with model support.
//!
//! Provides unified interface for spawning machines with either GLTF models
//! or procedural fallback geometry.

use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_mesh::Mesh;
use bevy_pbr::StandardMaterial;
use bevy_scene::SceneRoot;
use bevy_transform::components::Transform;

use crate::machines::{
    BladeState, Machine, MachineActivity, MachineCatalog, MachineType, Mobility, PlayerControlled,
    WorkEnvelope,
};

use super::procedural::{
    spawn_procedural_dozer, spawn_procedural_dump_truck, spawn_procedural_excavator,
    spawn_procedural_loader,
};
use super::registry::{MachineModelRegistry, ModelAssets, ModelLoadState};

/// Marker component indicating a machine entity.
#[derive(Component, Default)]
pub struct MachineEntity;

/// Resource for spawning machines with proper visuals.
#[derive(Resource)]
pub struct MachineSpawner;

impl MachineSpawner {
    /// Spawns a machine at the given position with the appropriate visuals.
    ///
    /// Uses GLTF model if available, otherwise falls back to procedural geometry.
    pub fn spawn(
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        registry: &MachineModelRegistry,
        model_assets: &ModelAssets,
        catalog: &MachineCatalog,
        machine_type: MachineType,
        id: String,
        position: Vec3,
    ) -> Entity {
        // Get the machine bundle from catalog
        let bundle = catalog.create_machine(machine_type, id.clone());

        // Spawn the base entity
        let entity = commands
            .spawn((
                Transform::from_translation(position),
                bundle.machine,
                bundle.envelope,
                bundle.mobility,
                bundle.activity,
                MachineEntity,
                Name::new(format!("{} - {}", machine_type.name(), id)),
            ))
            .id();

        // Add visuals based on what's available
        if registry.has_model(machine_type) {
            // Use GLTF model
            if let Some(scene) = model_assets.get_scene(machine_type) {
                let config = registry.get_config(machine_type);
                let scale = config.map(|c| c.scale).unwrap_or(1.0);
                let y_offset = config.map(|c| c.y_offset).unwrap_or(0.0);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn((
                        SceneRoot(scene),
                        Transform::from_xyz(0.0, y_offset, 0.0).with_scale(Vec3::splat(scale)),
                    ));
                });
            }
        } else {
            // Use procedural geometry
            spawn_procedural_for_type(commands, meshes, materials, entity, machine_type);
        }

        entity
    }

    /// Spawns a player-controlled bulldozer.
    pub fn spawn_player_dozer(
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        registry: &MachineModelRegistry,
        model_assets: &ModelAssets,
        catalog: &MachineCatalog,
        position: Vec3,
    ) -> Entity {
        let entity = Self::spawn(
            commands,
            meshes,
            materials,
            registry,
            model_assets,
            catalog,
            MachineType::Dozer,
            "player-dozer".to_string(),
            position,
        );

        // Add player control components
        commands.entity(entity).insert((
            PlayerControlled,
            BladeState {
                height: 0.0,
                load: 0.0,
                capacity: 8.0,
            },
        ));

        entity
    }
}

/// Spawns procedural geometry for a specific machine type.
fn spawn_procedural_for_type(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    entity: Entity,
    machine_type: MachineType,
) {
    match machine_type {
        MachineType::Dozer => spawn_procedural_dozer(commands, meshes, materials, entity),
        MachineType::Excavator => spawn_procedural_excavator(commands, meshes, materials, entity),
        MachineType::Loader => spawn_procedural_loader(commands, meshes, materials, entity),
        MachineType::DumpTruck => spawn_procedural_dump_truck(commands, meshes, materials, entity),
    }
}

/// Convenience function to spawn a machine with model support.
///
/// This is the main entry point for spawning machines in the game.
pub fn spawn_machine_with_model(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    registry: &MachineModelRegistry,
    model_assets: &ModelAssets,
    catalog: &MachineCatalog,
    machine_type: MachineType,
    id: String,
    position: Vec3,
) -> Entity {
    MachineSpawner::spawn(
        commands,
        meshes,
        materials,
        registry,
        model_assets,
        catalog,
        machine_type,
        id,
        position,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_machine_entity_marker() {
        // Just verify the marker component exists and is default
        let _marker = MachineEntity::default();
    }
}
