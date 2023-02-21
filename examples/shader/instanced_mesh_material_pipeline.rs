//! A shader that renders a mesh with material multiple times in one draw call. It does not solve instancing materials.

use std::{hash::Hash, marker::PhantomData};

use bevy::{
    core_pipeline::core_3d::{AlphaMask3d, Opaque3d, Transparent3d},
    ecs::{
        query::QueryItem,
        system::{lifetimeless::*, SystemParamItem},
    },
    pbr::{
        MaterialPipeline, MaterialPipelineKey, MeshPipelineKey, MeshUniform, RenderMaterials,
        SetMaterialBindGroup, SetMeshBindGroup, SetMeshViewBindGroup,
    },
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{GpuBufferInfo, MeshVertexBufferLayout},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            RenderPhase, SetItemPipeline, TrackedRenderPass,
        },
        render_resource::*,
        renderer::RenderDevice,
        view::{ExtractedView, NoFrustumCulling},
        RenderApp, RenderSet,
    },
};
use bytemuck::{Pod, Zeroable};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(InstancedMeshMaterialPipelinePlugin::<StandardMaterial>::default())
        .add_startup_system(setup)
        .run();
}

#[derive(Resource)]
struct InstancedMeshMaterialPipeline<M: Material> {
    pub material_pipeline: MaterialPipeline<M>,
}

impl<M> FromWorld for InstancedMeshMaterialPipeline<M>
where
    M: Material,
{
    fn from_world(world: &mut World) -> Self {
        let mut material_pipeline = MaterialPipeline::<M>::from_world(world);

        let asset_server = world.resource::<AssetServer>();
        let shader = asset_server.load("shaders/instanced_mesh.wgsl");

        material_pipeline.vertex_shader = Some(shader);

        Self { material_pipeline }
    }
}

impl<M: Material> SpecializedMeshPipeline for InstancedMeshMaterialPipeline<M>
where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    type Key = MaterialPipelineKey<M>;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.material_pipeline.specialize(key, layout)?;

        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: VertexFormat::Float32x3.size(),
            step_mode: VertexStepMode::Instance,
            attributes: vec![VertexAttribute {
                format: VertexFormat::Float32x3,
                offset: 0,
                shader_location: 10,
            }],
        });

        Ok(descriptor)
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
            material: materials.add(StandardMaterial {
                base_color: Color::BLUE,
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert((
            Instances(
                (1..=10)
                    .flat_map(|x| (1..=10).map(move |y| (x as f32 / 10.0, y as f32 / 10.0)))
                    .map(|(x, y)| Instance {
                        position: Vec3::new(x * 10.0 - 5.0, y * 10.0 - 5.0, 0.0),
                    })
                    .collect(),
            ),
            // NOTE: Frustum culling is done based on the Aabb of the Mesh and the GlobalTransform.
            // As the cube is at the origin, if its Aabb moves outside the view frustum, all the
            // instanced cubes will be culled.
            // The InstanceMaterialData contains the 'GlobalTransform' information for this custom
            // instancing, and that is not taken into account with the built-in frustum culling.
            // We must disable the built-in frustum culling by adding the `NoFrustumCulling` marker
            // component to avoid incorrect culling.
            NoFrustumCulling,
        ));

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(0.0, 0.0, 5.0),
        point_light: PointLight {
            intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
            color: Color::RED,
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
}

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Instance {
    position: Vec3,
}

#[derive(Component, Deref)]
struct Instances(Vec<Instance>);

impl ExtractComponent for Instances {
    type Query = &'static Self;
    type Filter = ();
    type Out = Self;

    fn extract_component(instances: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(Instances(instances.0.clone()))
    }
}

#[derive(Default)]
pub struct InstancedMeshMaterialPipelinePlugin<M> {
    marker: PhantomData<M>,
}

impl<M> Plugin for InstancedMeshMaterialPipelinePlugin<M>
where
    M: Material + Sync + Send + 'static,
    M::Data: PartialEq + Eq + Hash + Clone,
{
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<Instances>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Opaque3d, DrawMeshInstancedWithMaterial<M>>()
            .add_render_command::<AlphaMask3d, DrawMeshInstancedWithMaterial<M>>()
            .add_render_command::<Transparent3d, DrawMeshInstancedWithMaterial<M>>()
            .init_resource::<InstancedMeshMaterialPipeline<M>>()
            .init_resource::<SpecializedMeshPipelines<InstancedMeshMaterialPipeline<M>>>()
            .add_system(queue_instanced_meshes_with_material::<M>.in_set(RenderSet::Queue))
            .add_system(prepare_instance_buffers.in_set(RenderSet::Prepare));
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_instanced_meshes_with_material<M>(
    opaque_draw_functions: Res<DrawFunctions<Opaque3d>>,
    alpha_mask_draw_functions: Res<DrawFunctions<AlphaMask3d>>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    instanced_mesh_material_pipeline: Res<InstancedMeshMaterialPipeline<M>>,
    msaa: Res<Msaa>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedMeshMaterialPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_materials: Res<RenderMaterials<M>>,
    material_meshes: Query<(Entity, &MeshUniform, &Handle<Mesh>, &Handle<M>), With<Instances>>,
    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<Opaque3d>,
        &mut RenderPhase<AlphaMask3d>,
        &mut RenderPhase<Transparent3d>,
    )>,
) where
    M: Material,
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_instanced_mesh_with_opaque_material = opaque_draw_functions
        .read()
        .id::<DrawMeshInstancedWithMaterial<M>>();
    let draw_instanced_mesh_with_alpha_mask_material = alpha_mask_draw_functions
        .read()
        .id::<DrawMeshInstancedWithMaterial<M>>();
    let draw_instanced_mesh_with_transparent_material = transparent_draw_functions
        .read()
        .id::<DrawMeshInstancedWithMaterial<M>>();

    let msaa_key = MeshPipelineKey::from_msaa_samples(msaa.samples());

    for (view, mut opaque_phase, mut alpha_mask_phase, mut transparent_phase) in &mut views {
        let view_key = msaa_key | MeshPipelineKey::from_hdr(view.hdr);
        let rangefinder = view.rangefinder3d();
        for (entity, mesh_uniform, mesh_handle, material_handle) in &material_meshes {
            if let (Some(mesh), Some(material)) = (
                render_meshes.get(mesh_handle),
                render_materials.get(material_handle),
            ) {
                let mesh_key =
                    view_key | MeshPipelineKey::from_primitive_topology(mesh.primitive_topology);
                let pipeline = pipelines
                    .specialize(
                        &pipeline_cache,
                        &instanced_mesh_material_pipeline,
                        MaterialPipelineKey {
                            mesh_key,
                            bind_group_data: material.key.clone(),
                        },
                        &mesh.layout,
                    )
                    .unwrap();

                let distance =
                    rangefinder.distance(&mesh_uniform.transform) + material.properties.depth_bias;

                let alpha_mode = material.properties.alpha_mode;

                match alpha_mode {
                    AlphaMode::Opaque => {
                        opaque_phase.add(Opaque3d {
                            entity,
                            draw_function: draw_instanced_mesh_with_opaque_material,
                            pipeline,
                            distance,
                        });
                    }
                    AlphaMode::Mask(_) => {
                        alpha_mask_phase.add(AlphaMask3d {
                            entity,
                            draw_function: draw_instanced_mesh_with_alpha_mask_material,
                            pipeline,
                            distance,
                        });
                    }
                    AlphaMode::Blend
                    | AlphaMode::Premultiplied
                    | AlphaMode::Add
                    | AlphaMode::Multiply => {
                        transparent_phase.add(Transparent3d {
                            entity,
                            draw_function: draw_instanced_mesh_with_transparent_material,
                            pipeline,
                            distance,
                        });
                    }
                }
            }
        }
    }
}

#[derive(Component)]
pub struct InstanceBuffer {
    buffer: Buffer,
    length: usize,
}

fn prepare_instance_buffers(
    mut commands: Commands,
    query: Query<(Entity, &Instances)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, instances) in &query {
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instance data buffer"),
            contents: bytemuck::cast_slice(instances.as_slice()),
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            length: instances.len(),
        });
    }
}

type DrawMeshInstancedWithMaterial<M> = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMaterialBindGroup<M, 1>,
    SetMeshBindGroup<2>,
    DrawMeshInstanced,
);

pub struct DrawMeshInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawMeshInstanced {
    type Param = SRes<RenderAssets<Mesh>>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = (Read<Handle<Mesh>>, Read<InstanceBuffer>);

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        (mesh_handle, instance_buffer): (&'w Handle<Mesh>, &'w InstanceBuffer),
        meshes: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let gpu_mesh = match meshes.into_inner().get(mesh_handle) {
            Some(gpu_mesh) => gpu_mesh,
            None => return RenderCommandResult::Failure,
        };

        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        match &gpu_mesh.buffer_info {
            GpuBufferInfo::Indexed {
                buffer,
                index_format,
                count,
            } => {
                pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                pass.draw_indexed(0..*count, 0, 0..instance_buffer.length as u32);
            }
            GpuBufferInfo::NonIndexed { vertex_count } => {
                pass.draw(0..*vertex_count, 0..instance_buffer.length as u32);
            }
        }
        RenderCommandResult::Success
    }
}
