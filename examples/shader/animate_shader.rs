//! A shader that uses dynamic data like the time since startup.
//!
//! This example uses a material with a shared bind group.

use bevy::{
    pbr::SharedBindGroupPlugin,
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::FallbackImage,
        RenderApp, RenderStage,
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMaterialPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
) {
    // cube
    commands.spawn().insert_bundle(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(CustomMaterial { speed: 2.0 }),
        ..default()
    });

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(AsBindGroup, TypeUuid, Clone)]
#[uuid = "30446bcf-4507-4f14-ade7-1b8cd583664c"]
struct CustomMaterial {
    #[uniform(0)]
    speed: f32,
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/animate_shader.wgsl".into()
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractResourcePlugin::<ExtractedTime>::default())
            .add_plugin(SharedBindGroupPlugin::<TimeBindGroup>::default())
            .add_plugin(MaterialPlugin::<CustomMaterial, TimeBindGroup>::default());

        app.sub_app_mut(RenderApp)
            .init_resource::<TimeBindGroup>()
            .add_system_to_stage(RenderStage::Prepare, prepare_time_buffer);
    }
}

#[derive(Resource, Default)]
struct ExtractedTime {
    seconds_since_startup: f32,
}

impl ExtractResource for ExtractedTime {
    type Source = Time;

    fn extract_resource(time: &Self::Source) -> Self {
        ExtractedTime {
            seconds_since_startup: time.seconds_since_startup() as f32,
        }
    }
}

#[derive(Resource)]
struct TimeBindGroup {
    buffer: Buffer,
}

impl FromWorld for TimeBindGroup {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        Self {
            buffer: render_device.create_buffer(&BufferDescriptor {
                label: Some("time uniform buffer"),
                size: std::mem::size_of::<f32>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }
}

impl AsBindGroup for TimeBindGroup {
    type Data = ();

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        _images: &RenderAssets<Image>,
        _fallback_image: &FallbackImage,
    ) -> Result<PreparedBindGroup<Self>, AsBindGroupError> {
        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("time bind group"),
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: self.buffer.as_entire_binding(),
            }],
        });
        Ok(PreparedBindGroup {
            bindings: Vec::new(),
            bind_group,
            data: (),
        })
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("time bind group layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
                },
                count: None,
            }],
        })
    }
}

// write the extracted time into the corresponding uniform buffer
fn prepare_time_buffer(
    time: Res<ExtractedTime>,
    time_group: ResMut<TimeBindGroup>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &time_group.buffer,
        0,
        bevy::core::cast_slice(&[time.seconds_since_startup]),
    );
}
