//! This example illustrates how to create a texture for use with a `texture_2d_array<f32>` shader
//! uniform variable.

use bevy::{
    image::{ImageArrayLayout, ImageLoaderSettings},
    prelude::*,
    reflect::TypePath,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory
const SHADER_ASSET_PATH: &str = "shaders/array_texture.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ArrayTextureMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Load the texture.
    let array_texture = asset_server.load_with_settings(
        "textures/array_texture.png",
        |settings: &mut ImageLoaderSettings| {
            settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 4 });
        },
    );

    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
    ));

    // Spawn some cubes using the array texture
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(ArrayTextureMaterial { array_texture });
    for x in -5..=5 {
        commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            Transform::from_xyz(x as f32 + 0.5, 0.0, 0.0),
        ));
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ArrayTextureMaterial {
    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    array_texture: Handle<Image>,
}

impl Material for ArrayTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
