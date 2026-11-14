//! GPU-authored instance batches: a particle swarm whose transforms are
//! written by a compute shader. Move the mouse to attract the particles.
//! Press Space to pause simulation and publishing while continuing to render.

use std::borrow::Cow;

use bevy::{
    camera::{primitives::Aabb, Hdr},
    math::Vec3A,
    pbr::gpu_instance_batch::{
        GpuBatchedMesh3d, GpuInstanceBatchPlugin, GpuInstanceBatchReservations,
        GpuInstanceBatchSystems,
    },
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            binding_types::{storage_buffer_sized, uniform_buffer},
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, Buffer,
            BufferDescriptor, BufferId, BufferUsages, CachedComputePipelineId, CachedPipelineState,
            ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, ShaderStages,
            ShaderType, UniformBuffer,
        },
        renderer::{RenderContext, RenderDevice, RenderGraph, RenderQueue},
        sync_world::MainEntityHashMap,
        GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
    },
};

const SHADER_ASSET_PATH: &str = "shaders/gpu_particles_simulate.wesl";
const WORKGROUP_SIZE: u32 = 64;
const PARTICLES_PER_EMITTER: u32 = 4096;
const PARTICLE_STATE_SIZE: u64 = 32;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(GpuInstanceBatchPlugin)
        .add_plugins(GpuParticlesSimulationPlugin)
        .init_resource::<MouseWorldPos>()
        .init_resource::<SimulationPaused>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_mouse_world_pos, toggle_simulation))
        .run();
}

#[derive(Resource, Default, Clone, Copy, ExtractResource)]
#[extract_app(RenderApp)]
struct MouseWorldPos(Vec3);

#[derive(Resource, Default, Clone, ExtractResource)]
#[extract_app(RenderApp)]
struct SimulationPaused(bool);

fn toggle_simulation(keys: Res<ButtonInput<KeyCode>>, mut paused: ResMut<SimulationPaused>) {
    if keys.just_pressed(KeyCode::Space) {
        paused.0 = !paused.0;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(32.0, 32.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.1, 0.12),
            perceptual_roughness: 0.4,
            metallic: 0.2,
            ..default()
        })),
        Transform::from_xyz(0.0, -2.0, 0.0),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.5, 4.0, 1.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.28),
            perceptual_roughness: 0.35,
            metallic: 0.6,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let particle_mesh = meshes.add(Cuboid::new(0.22, 0.22, 0.22));
    let particle_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.78, 0.66),
        metallic: 0.0,
        perceptual_roughness: 0.55,
        reflectance: 0.3,
        ..default()
    });

    commands.spawn((
        GpuBatchedMesh3d {
            mesh: particle_mesh,
            max_capacity: PARTICLES_PER_EMITTER,
        },
        Aabb {
            center: Vec3A::ZERO,
            half_extents: Vec3A::splat(16.0),
        },
        MeshMaterial3d(particle_material),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 5_000.0,
            shadow_maps_enabled: true,
            color: Color::srgb(1.0, 0.95, 0.9),
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            color: Color::srgb(0.3, 0.6, 1.0),
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(-5.0, 3.0, -4.0),
    ));
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            color: Color::srgb(1.0, 0.4, 0.2),
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(5.0, 3.0, 4.0),
    ));
    commands.spawn((
        PointLight {
            intensity: 1_200_000.0,
            color: Color::srgb(0.8, 1.0, 0.6),
            range: 20.0,
            ..default()
        },
        Transform::from_xyz(0.0, 6.0, -6.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Hdr,
        Bloom::default(),
        Transform::from_xyz(0.0, 3.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
}

#[derive(Component)]
struct MainCamera;

fn update_mouse_world_pos(
    windows: Query<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut mouse_pos: ResMut<MouseWorldPos>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    if ray.direction.y.abs() < 1e-4 {
        return;
    }
    let t = -ray.origin.y / ray.direction.y;
    if t > 0.0 {
        mouse_pos.0 = ray.origin + ray.direction * t;
    }
}

struct GpuParticlesSimulationPlugin;

impl Plugin for GpuParticlesSimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<MouseWorldPos>::default(),
            ExtractResourcePlugin::<SimulationPaused>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<ParticleBatches>()
            .add_systems(RenderStartup, init_particle_sim_pipeline)
            .add_systems(
                Render,
                prepare_particle_sim_bind_groups.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(
                RenderGraph,
                dispatch_particle_sim.in_set(GpuInstanceBatchSystems::Publish),
            );
    }
}

#[derive(Resource)]
struct ParticleSimPipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    pipeline: CachedComputePipelineId,
}

#[derive(Copy, Clone, Default, ShaderType)]
struct ParticleSimParams {
    count: u32,
    time: f32,
    dt: f32,
    mouse_world_pos: Vec4,
}

#[derive(Resource, Default)]
struct ParticleBatches {
    per_batch: MainEntityHashMap<ParticleBatchState>,
}

struct ParticleBatchState {
    state: Buffer,
    params: UniformBuffer<ParticleSimParams>,
    /// Keyed by the output buffer it binds: resizing a reservation replaces
    /// that buffer.
    bind_group: Option<(BufferId, BindGroup)>,
}

fn init_particle_sim_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pipeline_cache: Res<PipelineCache>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "ParticleSimBindGroupLayout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                uniform_buffer::<ParticleSimParams>(false),
            ),
        ),
    );

    let shader = asset_server.load(SHADER_ASSET_PATH);
    let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("particle_sim_pipeline".into()),
        layout: vec![bind_group_layout.clone()],
        shader,
        entry_point: Some(Cow::from("simulate")),
        ..default()
    });

    commands.insert_resource(ParticleSimPipeline {
        bind_group_layout,
        pipeline,
    });
}

fn prepare_particle_sim_bind_groups(
    pipeline: Res<ParticleSimPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    reservations: Res<GpuInstanceBatchReservations>,
    time: Res<Time>,
    mouse_world_pos: Res<MouseWorldPos>,
    mut batches: ResMut<ParticleBatches>,
) {
    batches.per_batch.retain(|entity, batch| {
        reservations.get(*entity).is_some_and(|reservation| {
            batch.state.size() == reservation.max_capacity() as u64 * PARTICLE_STATE_SIZE
        })
    });

    if !matches!(
        pipeline_cache.get_compute_pipeline_state(pipeline.pipeline),
        CachedPipelineState::Ok(_)
    ) {
        return;
    }

    for (main_entity, reservation) in reservations.iter() {
        let batch = batches
            .per_batch
            .entry(main_entity)
            .or_insert_with(|| ParticleBatchState {
                // Zeroed state is initialized entirely by the compute shader.
                state: render_device.create_buffer(&BufferDescriptor {
                    label: Some("particle_state"),
                    size: reservation.max_capacity() as u64 * PARTICLE_STATE_SIZE,
                    usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                params: UniformBuffer::default(),
                bind_group: None,
            });

        batch.params.set(ParticleSimParams {
            count: reservation.max_capacity(),
            time: time.elapsed_secs(),
            dt: time.delta_secs().min(1.0 / 30.0),
            mouse_world_pos: mouse_world_pos.0.extend(0.0),
        });
        batch.params.write_buffer(&render_device, &render_queue);

        let output = reservation.output_buffer();
        if !matches!(&batch.bind_group, Some((id, _)) if *id == output.id()) {
            let bind_group = render_device.create_bind_group(
                Some("particle_sim_bind_group"),
                &pipeline_cache.get_bind_group_layout(&pipeline.bind_group_layout),
                &BindGroupEntries::sequential((
                    output.as_entire_binding(),
                    batch.state.as_entire_binding(),
                    batch.params.binding().unwrap(),
                )),
            );
            batch.bind_group = Some((output.id(), bind_group));
        }
    }
}

fn dispatch_particle_sim(
    mut render_context: RenderContext,
    paused: Res<SimulationPaused>,
    batches: Res<ParticleBatches>,
    pipeline: Res<ParticleSimPipeline>,
    pipeline_cache: Res<PipelineCache>,
) {
    if paused.0 || batches.per_batch.is_empty() {
        return;
    }
    let Some(compute_pipeline) = pipeline_cache.get_compute_pipeline(pipeline.pipeline) else {
        return;
    };

    let mut pass = render_context
        .command_encoder()
        .begin_compute_pass(&ComputePassDescriptor {
            label: Some("particle_sim"),
            timestamp_writes: None,
        });
    pass.set_pipeline(compute_pipeline);

    for batch in batches.per_batch.values() {
        let Some((_, bind_group)) = &batch.bind_group else {
            continue;
        };
        pass.set_bind_group(0, bind_group, &[]);
        pass.dispatch_workgroups(batch.params.get().count.div_ceil(WORKGROUP_SIZE), 1, 1);
    }
}
