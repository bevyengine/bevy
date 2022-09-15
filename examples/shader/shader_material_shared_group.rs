//! This example illustrates how to create multiple custom materials that share some
//! common data. Here two materials are created that both are aware of an illistrative
//! force field. The "Emitter" material uses the force field data to create a wobble
//! effect in its vertex shader and the "Receiver" material uses the info to create
//! a ripple effect in its fragment shader.

use bevy::{
    pbr::SharedBindGroupPlugin,
    prelude::*,
    reflect::TypeUuid,
    render::{
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::FallbackImage,
        Extract, RenderApp, RenderStage,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMaterialPlugin)
        .add_startup_system(setup)
        .add_system(keyboard_control)
        .run();
}

#[derive(Resource)]
struct SharedShaderFunctions(Handle<Shader>);

#[derive(Component)]
struct FieldEmitter {
    radius: f32,
    strength: f32,
    propagation_speed: f32,
    phase_speed: f32,
}

fn setup(
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut receiver_materials: ResMut<Assets<ReceiverMaterial>>,
    mut emitter_materials: ResMut<Assets<EmitterMaterial>>,
) {
    // Load shared shader functions and save the handle in a resource
    // to prevent it from being unloaded.
    commands.insert_resource(SharedShaderFunctions(
        asset_server.load("shaders/shared_group_common.wgsl"),
    ));

    // Light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Ground plane
    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 10.0 })),
        material: receiver_materials.add(ReceiverMaterial {
            base_color: Color::SEA_GREEN,
        }),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });

    // Red cube
    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: receiver_materials.add(ReceiverMaterial {
            base_color: Color::ORANGE_RED,
        }),
        transform: Transform::from_xyz(-2.0, 0.5, 0.0),
        ..default()
    });

    // Yellow capsule
    commands.spawn_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Capsule {
            radius: 0.5,
            ..default()
        })),
        material: receiver_materials.add(ReceiverMaterial {
            base_color: Color::YELLOW,
        }),
        transform: Transform::from_xyz(2.0, 0.5, 0.0),
        ..default()
    });

    // Field emitter
    commands
        .spawn_bundle(MaterialMeshBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 0.5,
                ..default()
            })),
            material: emitter_materials.add(EmitterMaterial {
                base_color: Color::BLUE,
            }),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        })
        .insert(FieldEmitter {
            radius: 2.0,
            strength: 1.0,
            phase_speed: 35.0,
            propagation_speed: 10.0,
        });

    // Camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 1.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    println!("Sphere movement controls:");
    println!("  - W/S/A/D: Move in/out/left/right");
    println!("  - Q/E: Move up/down");
}

fn keyboard_control(
    time: Res<Time>,
    keyboard: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<FieldEmitter>>,
) {
    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::A) {
        movement.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::D) {
        movement.x += 1.0;
    }
    if keyboard.pressed(KeyCode::W) {
        movement.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::S) {
        movement.z += 1.0;
    }
    if keyboard.pressed(KeyCode::Q) {
        movement.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::E) {
        movement.y += 1.0;
    }

    let mut transform = query.single_mut();
    transform.translation += movement * time.delta_seconds();
}

struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(SharedBindGroupPlugin::<SharedShaderData>::default())
            .add_plugin(MaterialPlugin::<ReceiverMaterial, SharedShaderData>::default())
            .add_plugin(MaterialPlugin::<EmitterMaterial, SharedShaderData>::default());

        app.sub_app_mut(RenderApp)
            .init_resource::<ExtractedTime>()
            .init_resource::<ExtractedEmitter>()
            .init_resource::<SharedShaderData>()
            .add_system_to_stage(RenderStage::Extract, extract_shared_data)
            .add_system_to_stage(RenderStage::Prepare, prepare_shared_data);
    }
}

#[derive(AsBindGroup, TypeUuid, Clone, Default)]
#[uuid = "f1f87b22-eddd-4df4-8aa0-eef2c2eb0ae9"]
struct EmitterMaterial {
    #[uniform(0)]
    base_color: Color,
}

impl Material for EmitterMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/shared_group_mat1.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/shared_group_mat1.wgsl".into()
    }
}

#[derive(AsBindGroup, TypeUuid, Clone, Default)]
#[uuid = "9e26eeeb-cb96-45f6-85f0-a749b2d6f6dc"]
struct ReceiverMaterial {
    #[uniform(0)]
    base_color: Color,
}

impl Material for ReceiverMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/shared_group_mat2.wgsl".into()
    }
}

#[derive(Resource, Default, Clone, ShaderType)]
struct ExtractedTime {
    seconds_since_startup: f32,
}

#[derive(Resource, Default, Clone, ShaderType)]
struct ExtractedEmitter {
    position: Vec3,
    radius: f32,
    strength: f32,
    propagation_speed: f32,
    phase_speed: f32,
}

#[derive(Resource, Default)]
struct SharedShaderData {
    time_buffer: UniformBuffer<ExtractedTime>,
    emitter_buffer: UniformBuffer<ExtractedEmitter>,
}

impl AsBindGroup for SharedShaderData {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        _images: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        if let (Some(time_binding), Some(emitter_binding)) =
            (self.time_buffer.binding(), self.emitter_buffer.binding())
        {
            Ok(PreparedBindGroup {
                bindings: Vec::new(),
                bind_group: render_device.create_bind_group(&BindGroupDescriptor {
                    label: None,
                    layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: time_binding,
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: emitter_binding,
                        },
                    ],
                }),
                data: (),
            })
        } else {
            Err(AsBindGroupError::RetryNextUpdate)
        }
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("time bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ExtractedTime::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(ExtractedEmitter::min_size()),
                    },
                    count: None,
                },
            ],
        })
    }
}

fn extract_shared_data(
    time_source: Extract<Res<Time>>,
    mut extracted_time: ResMut<ExtractedTime>,
    query: Extract<Query<(&GlobalTransform, &FieldEmitter)>>,
    mut extracted_emitter: ResMut<ExtractedEmitter>,
) {
    *extracted_time = ExtractedTime {
        seconds_since_startup: time_source.seconds_since_startup() as f32,
    };

    let (transform, field) = query.single();
    *extracted_emitter = ExtractedEmitter {
        position: transform.translation(),
        radius: field.radius,
        strength: field.strength,
        propagation_speed: field.propagation_speed,
        phase_speed: field.phase_speed,
    }
}

fn prepare_shared_data(
    time: Res<ExtractedTime>,
    emitter: Res<ExtractedEmitter>,
    mut shared_shader_data: ResMut<SharedShaderData>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    shared_shader_data.time_buffer.set(time.clone());
    shared_shader_data
        .time_buffer
        .write_buffer(&render_device, &render_queue);

    shared_shader_data.emitter_buffer.set(emitter.clone());
    shared_shader_data
        .emitter_buffer
        .write_buffer(&render_device, &render_queue);
}