use bevy::{
    core_pipeline::Transparent3d,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    pbr::{
        DrawMesh, MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup,
        SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayout,
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, EntityRenderCommand, RenderCommandResult, RenderPhase,
            SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        view::{ComputedVisibility, ExtractedView, Msaa, Visibility},
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

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    // cube
    commands.spawn().insert_bundle((
        meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        Transform::from_xyz(0.0, 0.5, 0.0),
        GlobalTransform::default(),
        CustomMaterial,
        Visibility::default(),
        ComputedVisibility::default(),
    ));

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct CustomMaterial;

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>();
        let buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("time uniform buffer"),
            size: std::mem::size_of::<f32>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .insert_resource(TimeMeta {
                buffer,
                bind_group: None,
            })
            .init_resource::<CustomPipeline>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_time)
            .add_system_to_stage(RenderStage::Extract, extract_custom_material)
            .add_system_to_stage(RenderStage::Prepare, prepare_time)
            .add_system_to_stage(RenderStage::Queue, queue_custom)
            .add_system_to_stage(RenderStage::Queue, queue_time_bind_group);
    }
}

// extract the `CustomMaterial` component into the render world
fn extract_custom_material(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    mut query: Query<Entity, With<CustomMaterial>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for entity in query.iter_mut() {
        values.push((entity, (CustomMaterial,)));
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

// add each entity with a mesh and a `CustomMaterial` to every view's `Transparent3d` render phase using the `CustomPipeline`
#[allow(clippy::too_many_arguments)]
fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<CustomMaterial>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    let key = MeshPipelineKey::from_msaa_samples(msaa.samples)
        | MeshPipelineKey::from_primitive_topology(PrimitiveTopology::TriangleList);

    for (view, mut transparent_phase) in views.iter_mut() {
        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (entity, mesh_uniform, mesh_handle) in material_meshes.iter() {
            if let Some(mesh) = render_meshes.get(mesh_handle) {
                let pipeline = pipelines
                    .specialize(&mut pipeline_cache, &custom_pipeline, key, &mesh.layout)
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}

#[derive(Default)]
struct ExtractedTime {
    seconds_since_startup: f32,
}

// extract the passed time into a resource in the render world
fn extract_time(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(ExtractedTime {
        seconds_since_startup: time.seconds_since_startup() as f32,
    });
}

struct TimeMeta {
    buffer: Buffer,
    bind_group: Option<BindGroup>,
}

// write the extracted time into the corresponding uniform buffer
fn prepare_time(
    time: Res<ExtractedTime>,
    time_meta: ResMut<TimeMeta>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(
        &time_meta.buffer,
        0,
        bevy::core::cast_slice(&[time.seconds_since_startup]),
    );
}

// create a bind group for the time uniform buffer
fn queue_time_bind_group(
    render_device: Res<RenderDevice>,
    mut time_meta: ResMut<TimeMeta>,
    pipeline: Res<CustomPipeline>,
) {
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.time_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: time_meta.buffer.as_entire_binding(),
        }],
    });
    time_meta.bind_group = Some(bind_group);
}

pub struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    time_bind_group_layout: BindGroupLayout,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("shaders/animate_shader.wgsl");

        let render_device = world.get_resource_mut::<RenderDevice>().unwrap();
        let time_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("time bind group"),
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
            });

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        CustomPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
            time_bind_group_layout,
        }
    }
}

impl SpecializedMeshPipeline for CustomPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
            self.time_bind_group_layout.clone(),
        ]);
        Ok(descriptor)
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    SetTimeBindGroup<2>,
    DrawMesh,
);

struct SetTimeBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetTimeBindGroup<I> {
    type Param = SRes<TimeMeta>;

    fn render<'w>(
        _view: Entity,
        _item: Entity,
        time_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let time_bind_group = time_meta.into_inner().bind_group.as_ref().unwrap();
        pass.set_bind_group(I, time_bind_group, &[]);

        RenderCommandResult::Success
    }
}
