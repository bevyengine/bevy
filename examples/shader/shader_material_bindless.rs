//! A material that uses bindless textures.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

const SHADER_ASSET_PATH: &str = "shaders/bindless_material.wgsl";

// `#[bindless(limit(4))]` indicates that we want Bevy to group materials into
// bind groups of at most 4 materials each.
// Note that we use the structure-level `#[uniform]` attribute to supply
// ordinary data to the shader.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[uniform(0, BindlessMaterialUniform, binding_array(10))]
#[bindless(limit(4))]
struct BindlessMaterial {
    color: LinearRgba,
    // This will be exposed to the shader as a binding array of 4 textures and a
    // binding array of 4 samplers.
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

// This buffer will be presented to the shader as `@binding(10)`.
#[derive(ShaderType)]
struct BindlessMaterialUniform {
    color: LinearRgba,
}

impl<'a> From<&'a BindlessMaterial> for BindlessMaterialUniform {
    fn from(material: &'a BindlessMaterial) -> Self {
        BindlessMaterialUniform {
            color: material.color,
        }
    }
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
