//! A material that uses bindless textures.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

const SHADER_ASSET_PATH: &str = "shaders/bindless_material.wgsl";

// `#[bindless(4)]` indicates that we want Bevy to group materials into bind
// groups of at most 4 materials each.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bindless(4)]
struct BindlessMaterial {
    // This will be exposed to the shader as a binding array of 4 *storage*
    // buffers (as bindless uniforms don't exist).
    #[uniform(0)]
    color: LinearRgba,
    // This will be exposed to the shader as a binding array of 4 textures and a
    // binding array of 4 samplers.
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

// The entry point.
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<BindlessMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

// Creates a simple scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Add a cube with a blue tinted texture.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::BLUE,
            color_texture: Some(asset_server.load("branding/bevy_logo_dark.png")),
        })),
        Transform::from_xyz(-2.0, 0.5, 0.0),
    ));

    // Add a cylinder with a red tinted texture.
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::RED,
            color_texture: Some(asset_server.load("branding/bevy_logo_light.png")),
        })),
        Transform::from_xyz(2.0, 0.5, 0.0),
    ));

    // Add a camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

impl Material for BindlessMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
