//! Demonstrates how to trigger various rendering errors, and how bevy can recover from them.

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            BufferDescriptor, BufferUsages, CommandEncoderDescriptor, ComputePassDescriptor,
            Extent3d, PipelineLayoutDescriptor, PollType, RawComputePipelineDescriptor,
            ShaderModuleDescriptor, ShaderSource, TextureDescriptor, TextureDimension,
            TextureFormat, TextureUsages,
        },
        renderer::{RenderDevice, RenderQueue},
        Render, RenderApp,
    },
};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        ExtractResourcePlugin::<RenderError>::default(),
    ))
    .add_systems(Startup, setup)
    .add_systems(Update, (update_camera, input))
    .init_resource::<RenderError>()
    .sub_app_mut(RenderApp)
    .add_systems(Render, cause_error);
    app.run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 1.0, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // help text
    commands.spawn((
        Text::new(
            "Press O to trigger an OutOfMemory error\n\
            Press V to trigger a Validation error\n\
            Press D to Destroy the render device (causes device lost error)\n\
            Press L to Loop infinitely in a compute shader (causes device lost error)\n\
            ",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn update_camera(mut camera: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    for mut t in camera.iter_mut() {
        let (s, c) = ops::sin_cos(time.elapsed_secs() * 0.3);
        *t = Transform::from_xyz(s * 10.0, 4.5, c * 10.0).looking_at(Vec3::ZERO, Vec3::Y);
    }
}

#[derive(Resource, ExtractResource, Clone, Default)]
enum RenderError {
    #[default]
    None,
    OutOfMemory,
    Validation,
    DeviceLost,
    Loop,
}

fn input(keyboard_input: Res<ButtonInput<KeyCode>>, mut error: ResMut<RenderError>) {
    *error = RenderError::None;
    if keyboard_input.just_pressed(KeyCode::KeyO) {
        *error = RenderError::OutOfMemory;
    }
    if keyboard_input.just_pressed(KeyCode::KeyV) {
        *error = RenderError::Validation;
    }
    if keyboard_input.just_pressed(KeyCode::KeyD) {
        *error = RenderError::DeviceLost;
    }
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        *error = RenderError::Loop;
    }
}

fn cause_error(error: Res<RenderError>, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    match *error {
        RenderError::None => {}
        RenderError::OutOfMemory => {
            let mut textures = Vec::new();
            for _ in 0..64 {
                textures.push(device.create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: 8192,
                        height: 8192,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Rgba16Float,
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                }));
            }
        }
        RenderError::Validation => {
            device.create_buffer(&BufferDescriptor {
                label: None,
                size: 1 << 63,
                usage: BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });
        }
        RenderError::DeviceLost => {
            device.wgpu_device().destroy();
            device.poll(PollType::wait_indefinitely()).unwrap();
        }
        RenderError::Loop => {
            let sm = device.create_and_validate_shader_module(ShaderModuleDescriptor {
                label: Some("shader"),
                source: ShaderSource::Wgsl(
                    "@compute @workgroup_size(1, 1, 1) fn main() { loop {} }".into(),
                ),
            });

            let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("pipeline_layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

            let pipeline = device.create_compute_pipeline(&RawComputePipelineDescriptor {
                label: Some("pipeline"),
                layout: Some(&pipeline_layout),
                module: &sm,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
            {
                let mut cpass = encoder.begin_compute_pass(&ComputePassDescriptor::default());
                cpass.set_pipeline(&pipeline);
                cpass.dispatch_workgroups(1, 1, 1);
            }
            device.poll(PollType::wait_indefinitely()).unwrap();
            queue.submit([encoder.finish()]);
        }
    }
}
