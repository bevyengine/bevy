use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};
use bevy_render::render_asset::RenderAssetUsages;
use bevy_render::storage::Storage;

const SHADER_ASSET_PATH: &str = "shaders/storage.wgsl";

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
    mut buffers: ResMut<Assets<Storage>>,
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

    let colors = buffers.add(Storage::new(
        bytemuck::cast_slice(color_data.as_slice()),
        RenderAssetUsages::default(),
    ));

    // Create the custom material with the storage buffer
    let custom_material = CustomMaterial {
        color: LinearRgba::WHITE,
        colors,
    };

    let material_handle = materials.add(custom_material);
    commands.insert_resource(CustomMaterialHandle(material_handle.clone()));

    // Spawn cubes with the custom material
    for i in 0..5 {
        commands.spawn(MaterialMeshBundle {
            mesh: meshes.add(Cuboid::new(0.9, 0.9, 0.9)),
            transform: Transform::from_xyz(i as f32 - 2.0, 0.5, 0.0),
            material: material_handle.clone(),
            ..default()
        });
    }

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

// Update the material color by time
fn update(
    time: Res<Time>,
    material_handle: Res<CustomMaterialHandle>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    let mut material = materials.get_mut(&material_handle.0).unwrap();
    let strength = (time.elapsed_seconds() * 3.0).sin() * 0.5 + 0.5;
    material.color = LinearRgba::from(Color::linear_rgb(strength, strength, strength));
}

// Holds a handle to the custom material
#[derive(Resource)]
pub struct CustomMaterialHandle(Handle<CustomMaterial>);

// This struct defines the data that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[storage(1, read_only)]
    colors: Handle<Storage>,
}

impl Material for CustomMaterial {
    fn vertex_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}
