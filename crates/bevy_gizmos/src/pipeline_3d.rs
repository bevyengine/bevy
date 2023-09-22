use crate::{
    line_gizmo_vertex_buffer_layouts, DrawLineGizmo, GizmoConfig, LineGizmo,
    LineGizmoUniformBindgroupLayout, SetLineGizmoBindGroup, LINE_SHADER_HANDLE,
};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core_pipeline::core_3d::Transparent3d;

use bevy_ecs::{
    prelude::Entity,
    schedule::IntoSystemConfigs,
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup};
use bevy_render::{
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, RenderLayers, ViewTarget},
    Render, RenderApp, RenderSet,
};

pub struct LineGizmo3dPlugin;
impl Plugin for LineGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawLineGizmo3d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .add_systems(
                Render,
                queue_line_gizmos_3d
                    .in_set(RenderSet::Queue)
                    .after(prepare_assets::<LineGizmo>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LineGizmoPipeline>();
    }
}

#[derive(Clone, Resource)]
struct LineGizmoPipeline {
    mesh_pipeline: MeshPipeline,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        LineGizmoPipeline {
            mesh_pipeline: render_world.resource::<MeshPipeline>().clone(),
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineGizmoPipelineKey {
    mesh_key: MeshPipelineKey,
    strip: bool,
    perspective: bool,
}

impl SpecializedRenderPipeline for LineGizmoPipeline {
    type Key = LineGizmoPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![
            #[cfg(feature = "webgl")]
            "SIXTEEN_BYTE_ALIGNMENT".into(),
        ];

        if key.perspective {
            shader_defs.push("PERSPECTIVE".into());
        }

        let format = if key.mesh_key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let view_layout = if key.mesh_key.msaa_samples() == 1 {
            self.mesh_pipeline.view_layout.clone()
        } else {
            self.mesh_pipeline.view_layout_multisampled.clone()
        };

        let layout = vec![view_layout, self.uniform_layout.clone()];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: line_gizmo_vertex_buffer_layouts(key.strip),
            },
            fragment: Some(FragmentState {
                shader: LINE_SHADER_HANDLE,
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
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline".into()),
            push_constant_ranges: vec![],
        }
    }
}

type DrawLineGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_line_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    config: Res<GizmoConfig>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<Transparent3d>,
        Option<&RenderLayers>,
    )>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo3d>().unwrap();

    for (view, mut transparent_phase, render_layers) in &mut views {
        let render_layers = render_layers.copied().unwrap_or_default();
        if !config.render_layers.intersects(&render_layers) {
            continue;
        }

        let mesh_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        for (entity, handle) in &line_gizmos {
            let Some(line_gizmo) = line_gizmo_assets.get(handle) else {
                continue;
            };

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineGizmoPipelineKey {
                    mesh_key,
                    strip: line_gizmo.strip,
                    perspective: config.line_perspective,
                },
            );

            transparent_phase.add(Transparent3d {
                entity,
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
