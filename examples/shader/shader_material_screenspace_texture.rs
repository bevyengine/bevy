//! A shader that samples a texture with view-independent UV coordinates.

use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, MaterialPlugin::<CustomMaterial>::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .run();
}

#[derive(Component)]
struct MainCamera;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut custom_materials: ResMut<Assets<CustomMaterial>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: standard_materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Cuboid::default()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: custom_materials.add(CustomMaterial {
            texture: asset_server.load(
                "models/FlightHelmet/FlightHelmet_Materials_LensesMat_OcclusionRoughMetal.png",
            ),
        }),
        ..default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(4.0, 2.5, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        MainCamera,
    ));
}

fn rotate_camera(mut camera: Query<&mut Transform, With<MainCamera>>, time: Res<Time>) {
    let cam_transform = camera.single_mut().into_inner();

    cam_transform.rotate_around(
        Vec3::ZERO,
        Quat::from_axis_angle(Vec3::Y, 45f32.to_radians() * time.delta_seconds()),
    );
    cam_transform.look_at(Vec3::ZERO, Vec3::Y);
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material_screenspace_texture.wgsl".into()
    }
}
