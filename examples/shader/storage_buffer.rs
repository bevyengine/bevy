//! A shader that uses storage buffer
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            AsBindGroup, Buffer, BufferDescriptor, BufferUsages, Maintain, MapMode, ShaderRef,
        },
        renderer::{RenderDevice, RenderQueue},
        settings::{WgpuFeatures, WgpuSettings},
        RenderApp, RenderStage,
    },
};
use bytemuck::cast_slice;

const BUFFER_SIZE: usize = 4;

fn main() {
    let mut app = App::new();

    app.insert_resource(WgpuSettings {
        features: WgpuFeatures::MAPPABLE_PRIMARY_BUFFERS,
        ..Default::default()
    })
    .add_plugins(DefaultPlugins)
    .add_plugin(ExtractResourcePlugin::<WritableBuffer>::default())
    .add_plugin(MaterialPlugin::<CustomMaterial>::default())
    .add_startup_system(setup)
    .add_system(update_buffer_system);

    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_system_to_stage(RenderStage::Cleanup, read_buffer_system);

    app.run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    render_device: ResMut<RenderDevice>,
) {
    // backing storage buffer
    let color_buffer = render_device.create_buffer(&BufferDescriptor {
        label: None,
        size: (std::mem::size_of::<f32>() * BUFFER_SIZE) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // writable buffer
    let writable_buffer = render_device.create_buffer(&BufferDescriptor {
        label: None,
        size: (std::mem::size_of::<f32>() * BUFFER_SIZE) as u64,
        usage: BufferUsages::STORAGE | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    // store as wrapped resource for later usage
    commands.insert_resource(ColorBuffer(color_buffer.clone()));
    commands.insert_resource(WritableBuffer(writable_buffer.clone()));

    // cube
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        material: materials.add(CustomMaterial {
            color: color_buffer,
            writable_buffer,
        }),
        ..default()
    });

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Deref, DerefMut, Resource)]
struct ColorBuffer(Buffer);

#[derive(Deref, DerefMut, Resource, Clone, ExtractResource)]
struct WritableBuffer(Buffer);

// updates the storage buffer
fn update_buffer_system(
    color_buffer: ResMut<ColorBuffer>,
    render_queue: Res<RenderQueue>,
    time: Res<Time>,
) {
    let blueness = (time.elapsed_seconds() * 5.).sin() / 2.0 + 0.5;
    render_queue.write_buffer(&color_buffer, 0, cast_slice(&[0.0, blueness, 0.0, 1.0]));
}

fn read_buffer_system(
    writable_buffer: ResMut<WritableBuffer>,
    render_device: ResMut<RenderDevice>,
) {
    let data = writable_buffer.slice(..);

    render_device.map_buffer(&data, MapMode::Read, move |val| {
        assert!(val.is_ok());
    });

    let device = render_device.wgpu_device();
    device.poll(Maintain::Wait);

    let data = data.get_mapped_range();
    let slice = unsafe { std::slice::from_raw_parts(data.as_ptr() as *const f32, BUFFER_SIZE) };

    info!("writable_buffer: {:?}", slice);

    drop(slice);
    drop(data);
    writable_buffer.unmap();
}

impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/storage_buffer.wgsl".into()
    }
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "b83c887a-ad95-4eb4-b150-60f95ef2028a"]
pub struct CustomMaterial {
    // read only storage buffer
    #[storage(0, read_only)]
    color: Buffer,

    // writable storage buffer (default)
    #[storage(1)]
    writable_buffer: Buffer,
}
