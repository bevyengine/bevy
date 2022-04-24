use bevy::camera::{FrameCapture, FrameCapturePlugin};
use bevy::core_pipeline::RenderTargetClearColors;
use bevy::prelude::*;
use bevy::render::camera::{CameraTypePlugin, RenderTarget};

use bevy::render::render_resource::TextureFormat;
use bevy::render::renderer::RenderDevice;

#[derive(Component, Default)]
pub struct CaptureCamera1;

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
        .add_system(animate_light_direction)
        .add_system(save_img)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut clear_colors: ResMut<RenderTargetClearColors>,
    render_device: Res<RenderDevice>,
) {
    commands.spawn_scene(asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"));

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_xyz(0.7, 0.7, 1.0)
                .looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            ..default()
        })
        .with_children(|parent| {
            let capture = FrameCapture::new_cpu_buffer(
                512,
                512,
                true,
                TextureFormat::Rgba8UnormSrgb,
                &mut images,
                &render_device,
            );
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
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.seconds_since_startup() as f32 * std::f32::consts::TAU / 10.0,
            -std::f32::consts::FRAC_PI_4,
        );
    }
}

pub fn save_img(captures: Query<&FrameCapture>, render_device: Res<RenderDevice>) {
    for (i, capture) in captures.iter().enumerate() {
        if let Some(target_buffer) = &capture.target_buffer {
            match target_buffer {
                bevy::camera::TargetBuffer::CPUBuffer(target_buffer) => {
                    target_buffer.get(&render_device, |buf| {
                        image::save_buffer(
                            format!("../test{i}.png"),
                            &buf,
                            capture.width,
                            capture.height,
                            image::ColorType::Rgba8,
                        )
                        .unwrap();
                    });
                }
                _ => continue,
            }
        }
    }
}
