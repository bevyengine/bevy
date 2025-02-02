//! This examples demonstrates using the `MeshInstanceIndex` component to sample from a
//! texture in a custom material.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::{
        mesh::MeshInstanceIndex,
        render_resource::{AsBindGroup, ShaderRef},
    },
};

const SHADER_ASSET_PATH: &str = "shaders/mesh_instance_index.wgsl";

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
    mut assets: ResMut<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let image = assets.load("branding/icon.png");

    // Create the custom material with the storage buffer
    let material_handle = materials.add(CustomMaterial {
        image: image.clone(),
    });

    // We're hardcoding the image dimensions for simplicity
    let image_dims = UVec2::new(256, 256);
    let total_pixels = image_dims.x * image_dims.y;

    for index in 0..total_pixels {
        // Get x,y from index - x goes left to right, y goes top to bottom
        let x = index % image_dims.x;
        let y = index / image_dims.x;

        // Convert to centered world coordinates
        let world_x = (x as f32 - image_dims.x as f32 / 2.0) / 50.0;
        let world_y = -((y as f32 - image_dims.y as f32 / 2.0) / 50.0); // Still need negative for world space

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.01)))),
            MeshMaterial3d(material_handle.clone()),
            MeshInstanceIndex(index),
            Transform::from_xyz(world_x, world_y, 0.0),
        ));
    }

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// Animate the transform
fn update(time: Res<Time>, mut transforms: Query<(&mut Transform, &MeshInstanceIndex)>) {
    for (mut transform, index) in transforms.iter_mut() {
        // Animate the z position based on time using the index to create a spiral
        transform.translation.z = (time.elapsed_secs() + index.0 as f32 * 0.01).sin();
    }
}

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[texture(0)]
    #[sampler(1)]
    image: Handle<Image>,
}

impl Material for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
