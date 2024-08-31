use super::{
    billboard_gizmo_vertex_buffer_layouts, BillboardGizmoUniformBindgroupLayout,
    DrawBillboardGizmo, GpuBillboardGizmo, SetBillboardGizmoBindGroup,
};
use crate::{config::GizmoMeshConfig, GizmoRenderSystem};
use bevy_app::{App, Plugin};
use bevy_core_pipeline::core_2d::{Transparent2d, CORE_2D_DEPTH_FORMAT};

use bevy_ecs::{
    prelude::Entity,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::FloatOrd;
use bevy_render::{
    render_asset::prepare_assets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, SetItemPipeline,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, RenderLayers, ViewTarget},
    Render, RenderApp,
};
use bevy_sprite::{Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dViewBindGroup};

use super::BILLBOARD_SHADER_HANDLE;

pub struct BillboardGizmo2dPlugin;

impl Plugin for BillboardGizmo2dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent2d, DrawBillboardGizmo2d>()
            .init_resource::<SpecializedRenderPipelines<BillboardGizmoPipeline>>()
            .add_systems(
                Render,
                queue_billboard_gizmos_2d
                    .in_set(GizmoRenderSystem::QueueGizmos2d)
                    .after(prepare_assets::<GpuBillboardGizmo>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<BillboardGizmoPipeline>();
    }
}

#[derive(Clone, Resource)]
struct BillboardGizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for BillboardGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        BillboardGizmoPipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            uniform_layout: render_world
                .resource::<BillboardGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct BillboardGizmoPipelineKey {
    mesh_key: Mesh2dPipelineKey,
}

impl SpecializedRenderPipeline for BillboardGizmoPipeline {
    type Key = BillboardGizmoPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let format = if key.mesh_key.contains(Mesh2dPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let shader_defs = vec![
            #[cfg(feature = "webgl")]
            "SIXTEEN_BYTE_ALIGNMENT".into(),
        ];

        let layout = vec![
            self.mesh_pipeline.view_layout.clone(),
            self.uniform_layout.clone(),
        ];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: BILLBOARD_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: billboard_gizmo_vertex_buffer_layouts(),
            },
            fragment: Some(FragmentState {
                shader: BILLBOARD_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_2D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: MultisampleState {
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("BillboardGizmo Pipeline 2D".into()),
            push_constant_ranges: vec![],
        }
    }
}

type DrawBillboardGizmo2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetBillboardGizmoBindGroup<1>,
    DrawBillboardGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_billboard_gizmos_2d(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<BillboardGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BillboardGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    billboard_gizmos: Query<(Entity, &GizmoMeshConfig)>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    mut views: Query<(Entity, &ExtractedView, &Msaa, Option<&RenderLayers>)>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawBillboardGizmo2d>()
        .unwrap();

    for (view_entity, view, msaa, render_layers) in &mut views {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        let render_layers = render_layers.unwrap_or_default();
        for (entity, config) in &billboard_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                BillboardGizmoPipelineKey { mesh_key },
            );

            transparent_phase.add(Transparent2d {
                entity,
                draw_function,
                pipeline,
                sort_key: FloatOrd(f32::INFINITY),
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}
