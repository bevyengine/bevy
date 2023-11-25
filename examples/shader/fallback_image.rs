//! This example tests that all texture dimensions are supported by
//! `FallbackImage`.
//!
//! When running this example, you should expect to see a window that only draws
//! the clear color. The test material does not shade any geometry; this example
//! only tests that the images are initialized and bound so that the app does
//! not panic.
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<FallbackTestMaterial>::default(),
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<FallbackTestMaterial>>,
) {
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(FallbackTestMaterial {
            image_1d: None,
            image_2d: None,
            image_2d_array: None,
            image_cube: None,
            image_cube_array: None,
            image_3d: None,
        }),
        ..Default::default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::new(1.5, 0.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

#[derive(AsBindGroup, Debug, Clone, Asset, TypePath)]
struct FallbackTestMaterial {
    #[texture(0, dimension = "1d")]
    #[sampler(1)]
    image_1d: Option<Handle<Image>>,

    #[texture(2, dimension = "2d")]
    #[sampler(3)]
    image_2d: Option<Handle<Image>>,

    #[texture(4, dimension = "2d_array")]
    #[sampler(5)]
    image_2d_array: Option<Handle<Image>>,

    #[texture(6, dimension = "cube")]
    #[sampler(7)]
    image_cube: Option<Handle<Image>>,

    #[texture(8, dimension = "cube_array")]
    #[sampler(9)]
    image_cube_array: Option<Handle<Image>>,

    #[texture(10, dimension = "3d")]
    #[sampler(11)]
    image_3d: Option<Handle<Image>>,
}

impl Material for FallbackTestMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/fallback_image_test.wgsl".into()
    }
}
