use bevy::{
    core_pipeline::Transparent3d,
    ecs::system::{lifetimeless::*, SystemParamItem},
    math::{prelude::*, Vec4Swizzles},
    pbr::{MeshPipeline, MeshPipelineKey, MeshUniform, SetMeshBindGroup, SetMeshViewBindGroup},
    prelude::*,
    render::{
        camera::Camera3d,
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_component::{ExtractComponent, ExtractComponentPlugin},
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
use bytemuck::{Pod, Zeroable};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(CustomMaterialPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let mesh_handle = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));
    for x in 1..=10 {
        for y in 1..=10 {
            let (x, y) = (x as f32 / 10.0, y as f32 / 10.0);
            commands.spawn_bundle((
                mesh_handle.clone(),
                // NOTE: The x-component of the scale is being used for the scale of the instance.
                // This would break if rotations are applied to the transform, but in that case you
                // would probably extend the instance data to account for it.
                Transform::from_xyz(x * 10.0 - 5.0, y * 10.0 - 5.0, 0.0).with_scale(Vec3::new(
                    (x * y).sqrt(),
                    0.0,
                    0.0,
                )),
                GlobalTransform::default(),
                ColorMaterialInstanced(Color::hsla(x * 360., y, 0.5, 1.0)),
                Visibility::default(),
                ComputedVisibility::default(),
            ));
        }
    }

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Clone, Component, Debug, Deref)]
struct ColorMaterialInstanced(Color);

impl ExtractComponent for ColorMaterialInstanced {
    type Query = &'static ColorMaterialInstanced;
    type Filter = ();

    fn extract_component(item: bevy::ecs::query::QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

pub struct CustomMaterialPlugin;

impl Plugin for CustomMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<ColorMaterialInstanced>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawCustom>()
            .init_resource::<CustomPipeline>()
            .init_resource::<SpecializedMeshPipelines<CustomPipeline>>()
            .add_system_to_stage(RenderStage::Prepare, prepare_instance_buffers)
            .add_system_to_stage(RenderStage::Queue, queue_custom);
    }
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct InstanceData {
    position: Vec3,
    scale: f32,
    color: [f32; 4],
}

#[derive(Component, Deref, DerefMut)]
pub struct InstanceBuffer(BufferVec<InstanceData>);

impl Default for InstanceBuffer {
    fn default() -> Self {
        Self(BufferVec::<InstanceData>::new(
            BufferUsages::VERTEX | BufferUsages::COPY_DST,
        ))
    }
}

#[derive(Component, Debug, Deref)]
pub struct InstanceIndex(u32);

fn prepare_instance_buffers(
    mut commands: Commands,
    views: Query<Entity, With<Camera3d>>,
    query: Query<(Entity, &MeshUniform, &ColorMaterialInstanced)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for view in views.iter() {
        let mut instance_buffer = InstanceBuffer::default();
        instance_buffer.reserve(query.iter().len(), &*render_device);
        for (entity, mesh_uniform, color_material_instanced) in query.iter() {
            let index = instance_buffer.push(InstanceData {
                position: mesh_uniform.transform.w_axis.xyz(),
                // NOTE: Using the x component of the scale as the instance scale
                scale: mesh_uniform.transform.x_axis.x,
                color: color_material_instanced.as_rgba_f32(),
            });
            commands.entity(entity).insert(InstanceIndex(index as u32));
        }
        instance_buffer.write_buffer(&*render_device, &*render_queue);
        commands.entity(view).insert(instance_buffer);
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_custom(
    transparent_3d_draw_functions: Res<DrawFunctions<Transparent3d>>,
    custom_pipeline: Res<CustomPipeline>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<CustomPipeline>>,
    mut pipeline_cache: ResMut<PipelineCache>,
    meshes: Res<RenderAssets<Mesh>>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>), With<ColorMaterialInstanced>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent3d>)>,
) {
    let draw_custom = transparent_3d_draw_functions
        .read()
        .get_id::<DrawCustom>()
        .unwrap();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples);

    for (view, mut transparent_phase) in views.iter_mut() {
        let inverse_view_matrix = view.transform.compute_matrix().inverse();
        let inverse_view_row_2 = inverse_view_matrix.row(2);
        for (entity, mesh_uniform, mesh_handle) in material_meshes.iter() {
            if let Some(mesh) = meshes.get(mesh_handle) {
                let key =
                    msaa_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(&mut pipeline_cache, &custom_pipeline, key, &mesh.layout)
                    .unwrap();
                transparent_phase.add(Transparent3d {
                    entity,
                    pipeline,
                    draw_function: draw_custom,
                    distance: inverse_view_row_2.dot(mesh_uniform.transform.col(3)),
                });
            }
        }
    }
}

pub struct CustomPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
}

impl FromWorld for CustomPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        asset_server.watch_for_changes().unwrap();
        let shader = asset_server.load("shaders/instancing.wgsl");

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        CustomPipeline {
            shader,
            mesh_pipeline: mesh_pipeline.clone(),
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
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: std::mem::size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3, // shader locations 0-2 are taken up by Position, Normal and UV attributes
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        descriptor.layout = Some(vec![
            self.mesh_pipeline.view_layout.clone(),
            self.mesh_pipeline.mesh_layout.clone(),
        ]);

        Ok(descriptor)
    }
}

type DrawCustom = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;
impl EntityRenderCommand for DrawMeshInstanced {
    type Param = (
        SRes<RenderAssets<Mesh>>,
        SQuery<(Read<Handle<Mesh>>, Read<InstanceIndex>)>,
        SQuery<Read<InstanceBuffer>>,
    );
    #[inline]
    fn render<'w>(
        view: Entity,
        item: Entity,
        (meshes, mesh_query, instance_buffer_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (mesh_handle, instance_index) = mesh_query.get(item).unwrap();
        let instance_buffer = instance_buffer_query.get_inner(view).unwrap();

        let gpu_mesh = match meshes.into_inner().get(mesh_handle) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer().unwrap().slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, instance_index.0..(instance_index.0 + 1));
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                pass.draw(0..*vertex_count, instance_index.0..(instance_index.0 + 1));
            }
        }
        RenderCommandResult::Success
    }
}
