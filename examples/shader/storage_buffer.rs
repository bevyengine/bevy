//! This example demonstrates how to use a storage buffer with `AsBindGroup` in a custom material.
use std::array;

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        storage::ShaderStorageBuffer,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/storage_buffer.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // Example data for the storage buffer
    let color_data: Vec<[f32; 4]> = vec![
        [1.0, 0.0, 0.0, 1.0],
        [0.0, 1.0, 0.0, 1.0],
        [0.0, 0.0, 1.0, 1.0],
        [1.0, 1.0, 0.0, 1.0],
        [0.0, 1.0, 1.0, 1.0],
    ];

    let colors = buffers.add(ShaderStorageBuffer::from(color_data));

    // Create the custom material with the storage buffer
    let material_handles: [Handle<CustomMaterial>; 5] = array::from_fn(|color_id| {
        materials.add(CustomMaterial {
            colors: colors.clone(),
            color_id: color_id as u32,
        })
    });

    commands.insert_resource(CustomMaterialHandles(material_handles.clone()));

    // Spawn cubes with the custom material
    let mut current_color_id = 0;
    for i in -6..=6 {
        for j in -3..=3 {
            commands.spawn((
                Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.3)))),
                MeshMaterial3d(material_handles[current_color_id % 5].clone()),
                Transform::from_xyz(i as f32, j as f32, 0.0),
            ));
            current_color_id += 1;
        }
    }

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// Update the material color by time
fn update(
    time: Res<Time>,
    material_handles: Res<CustomMaterialHandles>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    // All materials use the same buffer, so we only need to update one of them.
    // But we do need to at least mark the others as changed, so that Bevy will
    // reupload their contents to the GPU.
    for material in &material_handles.0[1..] {
        materials.get_mut(material);
    }
    let material = materials.get_mut(&material_handles.0[0]).unwrap();

    let buffer = buffers.get_mut(&material.colors).unwrap();
    buffer.set_data(
        (0..5)
            .map(|i| {
                let t = time.elapsed_secs() * 5.0;
                [
                    ops::sin(t + i as f32) / 2.0 + 0.5,
                    ops::sin(t + i as f32 + 2.0) / 2.0 + 0.5,
                    ops::sin(t + i as f32 + 4.0) / 2.0 + 0.5,
                    1.0,
                ]
            })
            .collect::<Vec<[f32; 4]>>()
            .as_slice(),
    );
}

// Holds handles to the custom materials
#[derive(Resource)]
struct CustomMaterialHandles([Handle<CustomMaterial>; 5]);

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[storage(0, read_only)]
    colors: Handle<ShaderStorageBuffer>,
    #[uniform(1)]
    color_id: u32,
}

impl Material for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
