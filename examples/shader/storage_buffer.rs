//! This example demonstrates how to use a storage buffer with `AsBindGroup` in a custom material.
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_render::render_asset::RenderAssetUsages;
use bevy_render::storage::ShaderStorageBuffer;

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

    let colors = buffers.add(ShaderStorageBuffer::new(
        bytemuck::cast_slice(color_data.as_slice()),
        RenderAssetUsages::default(),
    ));

    // Create the custom material with the storage buffer
    let custom_material = CustomMaterial { colors };

    let material_handle = materials.add(custom_material);
    commands.insert_resource(CustomMaterialHandle(material_handle.clone()));

    // Spawn cubes with the custom material
    for i in 0..10 {
        for j in 0..10 {
            commands.spawn(MaterialMeshBundle {
                mesh: meshes.add(Cuboid::from_size(Vec3::splat(0.3))),
                material: material_handle.clone(),
                transform: Transform::from_xyz(i as f32 - 5.0, j as f32 - 5.0, 0.0),
                ..default()
            });
        }
    }

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// Update the material color by time
fn update(
    time: Res<Time>,
    material_handle: Res<CustomMaterialHandle>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    let material = materials.get_mut(&material_handle.0).unwrap();
    let buffer = buffers.get_mut(&material.colors).unwrap();
    buffer.data = Some(
        bytemuck::cast_slice(
            (0..5)
                .map(|i| {
                    let t = time.elapsed_seconds() * 5.0;
                    [
                        (t + i as f32).sin() / 2.0 + 0.5,
                        (t + i as f32 + 2.0).sin() / 2.0 + 0.5,
                        (t + i as f32 + 4.0).sin() / 2.0 + 0.5,
                        1.0,
                    ]
                })
                .collect::<Vec<[f32; 4]>>()
                .as_slice(),
        )
        .to_vec(),
    );
}

// Holds a handle to the custom material
#[derive(Resource)]
struct CustomMaterialHandle(Handle<CustomMaterial>);

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[storage(0, read_only)]
    colors: Handle<ShaderStorageBuffer>,
}

impl Material for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
