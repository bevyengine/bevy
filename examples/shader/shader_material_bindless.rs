//! A material that uses bindless textures.

use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};

const SHADER_ASSET_PATH: &str = "shaders/bindless_material.wgsl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bindless(4)]
struct BindlessMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<BindlessMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<BindlessMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::BLUE,
            color_texture: Some(asset_server.load("branding/bevy_logo_dark.png")),
        })),
        Transform::from_xyz(-2.0, 0.5, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cylinder::default())),
        MeshMaterial3d(materials.add(BindlessMaterial {
            color: LinearRgba::RED,
            color_texture: Some(asset_server.load("branding/bevy_logo_light.png")),
        })),
        Transform::from_xyz(2.0, 0.5, 0.0),
    ));

    // camera
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
