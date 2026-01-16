//! Shows that multiple instances of a cube are automatically instanced in one draw call
//! Try running this example in a graphics profiler and all the cubes should be only a single draw call.
//! Also demonstrates how to use `MeshTag` to use external data in a custom material.

use bevy::{
    mesh::MeshTag, prelude::*, reflect::TypePath, render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

const SHADER_ASSET_PATH: &str = "shaders/automatic_instancing.wgsl";

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

/// Sets up an instanced grid of cubes, where each cube is colored based on an image that is
/// sampled in the vertex shader. The cubes are then animated in a spiral pattern.
///
/// This example demonstrates one use of automatic instancing and how to use `MeshTag` to use
/// external data in a custom material. For example, here we use the "index" of each cube to
/// determine the texel coordinate to sample from the image in the shader.
fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // We will use this image as our external data for our material to sample from in the vertex shader
    let image = assets.load("branding/icon.png");

    // Our single mesh handle that will be instanced
    let mesh_handle = meshes.add(Cuboid::from_size(Vec3::splat(0.01)));

    // Create the custom material with a reference to our texture
    // Automatic instancing works with any Material, including the `StandardMaterial`.
    // This custom material is used to demonstrate the optional `MeshTag` feature.
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
            // For automatic instancing to take effect you need to
            // use the same mesh handle and material handle for each instance
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            // This is an optional component that can be used to help tie external data to a mesh instance
            MeshTag(index),
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
fn update(time: Res<Time>, mut transforms: Query<(&mut Transform, &MeshTag)>) {
    for (mut transform, index) in transforms.iter_mut() {
        // Animate the z position based on time using the index to create a spiral
        transform.translation.z = ops::sin(time.elapsed_secs() + index.0 as f32 * 0.01);
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
