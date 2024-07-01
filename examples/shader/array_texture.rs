//! This example illustrates how to create a texture for use with a `texture_2d_array<f32>` shader
//! uniform variable.

use bevy::{
    asset::LoadState,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
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
        .add_systems(Update, create_array_texture)
        .run();
}

#[derive(Resource)]
struct LoadingTexture {
    is_loaded: bool,
    handle: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Start loading the texture.
    commands.insert_resource(LoadingTexture {
        is_loaded: false,
        handle: asset_server.load("textures/array_texture.png"),
    });

    // light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_xyz(3.0, 2.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

fn create_array_texture(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut loading_texture: ResMut<LoadingTexture>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ArrayTextureMaterial>>,
) {
    if loading_texture.is_loaded
        || asset_server.load_state(loading_texture.handle.id()) != LoadState::Loaded
    {
        return;
    }
    loading_texture.is_loaded = true;
    let image = images.get_mut(&loading_texture.handle).unwrap();

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    image.reinterpret_stacked_2d_as_array(array_layers);

    // Spawn some cubes using the array texture
    let mesh_handle = meshes.add(Cuboid::default());
    let material_handle = materials.add(ArrayTextureMaterial {
        array_texture: loading_texture.handle.clone(),
    });
    for x in -5..=5 {
        commands.spawn(MaterialMeshBundle {
            mesh: mesh_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz(x as f32 + 0.5, 0.0, 0.0),
            ..Default::default()
        });
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
