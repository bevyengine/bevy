//! A shader and a material that uses it.

use bevy::{
    core_pipeline::core_3d::PrepassSettings,
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .insert_resource(AssetServerSettings {
            watch_for_changes: true,
            ..Default::default()
        })
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_plugin(MaterialPlugin::<DepthMaterial>::default())
        .add_startup_system(setup)
        .add_system(rotate)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut depth_materials: ResMut<Assets<DepthMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: std_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // depth plane
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            flip: false,
            size: Vec2::new(2.0, 2.0),
        })),
        material: depth_materials.add(DepthMaterial {}),
        transform: Transform::from_xyz(-1.0, 1.0, 2.0)
            .looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
        ..default()
    });

    //cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: std_materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        },
        Rotates,
    ));

    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(CustomMaterial {
            color: Vec3::ONE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        }),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(CustomMaterial {
            color: Vec3::ONE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Opaque,
        }),
        transform: Transform::from_xyz(1.0, 0.5, 0.0),
        ..default()
    });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        PrepassSettings::default(),
    ));
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Vec3,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/red.wgsl".into()
    }
}

#[derive(Component)]
struct Rotates;

fn rotate(mut q: Query<&mut Transform, With<Rotates>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        let rot =
            (time.seconds_since_startup().sin() * 0.5 + 0.5) as f32 * std::f32::consts::PI * 2.0;
        t.rotation = Quat::from_rotation_z(rot);
    }
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "0af99895-b96e-4451-bc12-c6b1c1c52750"]
pub struct DepthMaterial {}

impl Material for DepthMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/show_depth.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
