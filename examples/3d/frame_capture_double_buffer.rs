use bevy::camera::{FrameCapture, FrameCapturePlugin, TargetBuffer};
use bevy::core_pipeline::RenderTargetClearColors;
use bevy::prelude::*;
use bevy::render::camera::{CameraTypePlugin, RenderTarget};

use bevy::render::render_resource::TextureFormat;

#[derive(Component, Default)]
pub struct CaptureCamera1;

// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

// Marks the main pass cube, to which the texture is applied.
#[derive(Component)]
struct MainPassCube;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 }) // Use 4x MSAA
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(CameraTypePlugin::<CaptureCamera1>::default())
        .add_plugin(FrameCapturePlugin)
        .add_startup_system(setup)
        .add_system(cube_rotator_system)
        .add_system(rotator_system)
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut clear_colors: ResMut<RenderTargetClearColors>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let capture = FrameCapture::new_gpu_double_buffer(
        512,
        512,
        true,
        TextureFormat::Rgba8UnormSrgb,
        &mut images,
    );

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // The cube that will be rendered to the texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.25, 0.0)),
            ..default()
        })
        .insert(FirstPassCube);

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    let cube_size = 0.25;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    if let Some(TargetBuffer::GPUBuffer(buf)) = capture.clone().target_buffer {
        // This material has the texture that has been rendered.
        let material_handle = materials.add(StandardMaterial {
            base_color_texture: Some(buf),
            reflectance: 0.02,
            unlit: false,
            ..default()
        });
        // Main pass cube, with material containing the rendered first pass texture.
        commands
            .spawn_bundle(PbrBundle {
                mesh: cube_handle,
                material: material_handle,
                transform: Transform {
                    translation: Vec3::new(0.0, 0.5, 0.0),
                    rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                    ..default()
                },
                ..default()
            })
            .insert(MainPassCube);
    }

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        })
        .with_children(|parent| {
            let render_target = RenderTarget::Image(capture.gpu_image.clone());
            clear_colors.insert(render_target.clone(), Color::GRAY);
            parent
                .spawn_bundle(PerspectiveCameraBundle::<CaptureCamera1> {
                    camera: Camera {
                        target: render_target,
                        ..default()
                    },
                    ..PerspectiveCameraBundle::new()
                })
                .insert(capture);
        });
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.5 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_z(1.3 * time.delta_seconds());
    }
}

/// Rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.0 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_y(0.7 * time.delta_seconds());
    }
}
