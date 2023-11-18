use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_core_pipeline::{
    core_2d::Transparent2d,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    schedule::IntoSystemConfigs,
    system::{lifetimeless::SRes, Query, Res, ResMut, Resource, SystemParamItem, SystemState},
    world::{FromWorld, World},
};
use bevy_log::error;
use bevy_render::{
    batching::{batch_and_prepare_render_phase, write_batched_instance_buffer, GetBatchData},
    globals::GlobalsUniform,
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::{prepare_assets, RenderAssets},
    render_phase::*,
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget, ViewUniform, VisibleEntities},
    Render, RenderApp, RenderSet,
};
use bevy_sprite::{tonemapping_pipeline_key, Mesh2dPipelineKey, SetMesh2dViewBindGroup};
use bevy_utils::FloatOrd;

use crate::mesh_pipeline::{
    gizmo_mesh_shared::{get_batch_data, DrawGizmo, GizmoMeshShared, SetGizmoBindGroup},
    GizmoUniform, Immediate, RenderGizmoInstances,
};

pub struct GizmoMesh2dPlugin;

impl Plugin for GizmoMesh2dPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<SpecializedMeshPipelines<GizmoMesh2dPipeline>>()
                .add_render_command::<Transparent2d, DrawGizmo2d>()
                .add_systems(
                    Render,
                    (
                        queue_gizmos_2d
                            .in_set(RenderSet::QueueMeshes)
                            .after(prepare_assets::<Mesh>),
                        batch_and_prepare_render_phase::<Transparent2d, GizmoMesh2dPipeline>
                            .in_set(RenderSet::PrepareResources),
                        write_batched_instance_buffer::<GizmoMesh2dPipeline>
                            .in_set(RenderSet::PrepareResourcesFlush),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<GizmoMesh2dPipeline>();
    }
}

#[derive(Component)]
pub(super) struct Gizmo2d;

#[derive(Resource, Clone)]
struct GizmoMesh2dPipeline {
    pub gizmo_pipeline: GizmoMeshShared,
    pub view_layout: BindGroupLayout,
}

impl FromWorld for GizmoMesh2dPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<Res<RenderDevice>> = SystemState::new(world);
        let render_device = system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: Some(ViewUniform::min_size()),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GlobalsUniform::min_size()),
                    },
                    count: None,
                },
            ],
            label: Some("mesh2d_view_layout"),
        });

        GizmoMesh2dPipeline {
            view_layout,
            gizmo_pipeline: world.resource::<GizmoMeshShared>().clone(),
        }
    }
}

impl GetBatchData for GizmoMesh2dPipeline {
    type Param = SRes<RenderGizmoInstances>;
    type Query = Entity;
    type QueryFilter = With<Gizmo2d>;
    type CompareData = AssetId<Mesh>;
    type BufferData = GizmoUniform;

    fn get_batch_data(
        gizmo_instances: &SystemParamItem<Self::Param>,
        entity: &QueryItem<Self::Query>,
    ) -> (Self::BufferData, Option<Self::CompareData>) {
        get_batch_data(gizmo_instances, entity)
    }
}

impl SpecializedMeshPipeline for GizmoMesh2dPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut shader_defs = Vec::new();
        if key.contains(Mesh2dPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(Mesh2dPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            match method {
                Mesh2dPipelineKey::TONEMAP_METHOD_NONE => {
                    shader_defs.push("TONEMAP_METHOD_NONE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE => {
                    shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_ACES_FITTED => {
                    shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_AGX => {
                    shader_defs.push("TONEMAP_METHOD_AGX".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM => {
                    shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC => {
                    shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
                }
                Mesh2dPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE => {
                    shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
                }
                _ => {}
            }
            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(Mesh2dPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }
        let mut descriptor = self.gizmo_pipeline.get_descriptor(layout)?;

        descriptor.vertex.shader_defs.extend(shader_defs.clone());

        let fragment = descriptor.fragment.as_mut().unwrap();

        fragment.shader_defs.extend(shader_defs);

        let format = if key.contains(Mesh2dPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        fragment.targets.push(Some(ColorTargetState {
            format,
            blend: Some(BlendState::ALPHA_BLENDING),
            write_mask: ColorWrites::ALL,
        }));

        descriptor.layout.insert(0, self.view_layout.clone());
        descriptor.primitive.topology = key.primitive_topology();
        descriptor.multisample.count = key.msaa_samples();
        descriptor.label = Some("transparent_gizmo_mesh2d_pipeline".into());

        Ok(descriptor)
    }
}

type DrawGizmo2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetGizmoBindGroup<1>,
    DrawGizmo,
);

fn queue_gizmos_2d(
    opaque_draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    pipeline: Res<GizmoMesh2dPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<GizmoMesh2dPipeline>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    mut render_mesh_instances: ResMut<RenderGizmoInstances>,
    mut views: Query<(
        &ExtractedView,
        &VisibleEntities,
        Option<&Tonemapping>,
        Option<&DebandDither>,
        &mut RenderPhase<Transparent2d>,
    )>,
    immediate: Query<Entity, With<Immediate>>,
) {
    for (view, visible_entities, tonemapping, dither, mut opaque_phase) in &mut views {
        let draw_opaque_pbr = opaque_draw_functions.read().id::<DrawGizmo2d>();

        let mut view_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= Mesh2dPipelineKey::TONEMAP_IN_SHADER;
                view_key |= tonemapping_pipeline_key(*tonemapping);
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= Mesh2dPipelineKey::DEBAND_DITHER;
            }
        }

        let visibile = visible_entities.entities.iter();
        // Mesh gizmos created via the immediate mode API won't be in the views VisibleEntities but may be visible.
        for entity in visibile.copied().chain(immediate.iter()) {
            let Some(gizmo_instance) = render_mesh_instances.get_mut(&entity) else {
                continue;
            };
            let Some(mesh) = render_meshes.get(gizmo_instance.mesh_asset_id) else {
                continue;
            };

            let mut mesh_key = view_key;

            mesh_key |= Mesh2dPipelineKey::from_primitive_topology(mesh.primitive_topology);

            let pipeline_id =
                pipelines.specialize(&pipeline_cache, &pipeline, mesh_key, &mesh.layout);
            let pipeline_id = match pipeline_id {
                Ok(id) => id,
                Err(err) => {
                    error!("{}", err);
                    continue;
                }
            };

            // + material.properties.depth_bias;
            opaque_phase.add(Transparent2d {
                entity,
                draw_function: draw_opaque_pbr,
                pipeline: pipeline_id,
                sort_key: FloatOrd(gizmo_instance.transform.translation.z),
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
