//! Minimal example demonstrating the Earthworks plugin.

use bevy::prelude::*;
use bevy_earthworks::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EarthworksPlugin::default())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut terrain: ResMut<VoxelTerrain>, config: Res<EarthworksConfig>) {
    // Spawn camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(20.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Spawn light
    commands.spawn((
        DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create some terrain chunks manually for testing
    use bevy_earthworks::terrain::{Chunk, ChunkCoord, DirtyChunk, MaterialId, Voxel};

    // Create a flat ground plane of dirt
    for cx in -2..=2 {
        for cz in -2..=2 {
            let mut chunk = Chunk::new();

            // Fill bottom half with dirt
            for y in 0..8 {
                for x in 0..16 {
                    for z in 0..16 {
                        chunk.set(x, y, z, Voxel::solid(MaterialId::Dirt));
                    }
                }
            }

            let coord = ChunkCoord::new(cx, 0, cz);
            let entity = commands.spawn((chunk, coord, DirtyChunk)).id();
            terrain.set_chunk_entity(coord, entity);
        }
    }

    println!("Earthworks minimal example started!");
    println!("Created {} terrain chunks", terrain.chunk_count());
}
