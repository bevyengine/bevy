use crate::{
    config::{GizmoLineJoins, GizmoMeshConfig},
    line_gizmo_vertex_buffer_layouts, line_join_gizmo_vertex_buffer_layouts, DrawLineGizmo,
    DrawLineJoinGizmo, GizmoRenderSystem, LineGizmo, LineGizmoUniformBindgroupLayout,
    SetLineGizmoBindGroup, LINE_JOINS_SHADER_HANDLE, LINE_SHADER_HANDLE,
};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core_pipeline::core_2d::Transparent2d;

use bevy_ecs::{
    prelude::Entity,
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs},
    system::{Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{AddRenderCommand, DrawFunctions, RenderPhase, SetItemPipeline},
    render_resource::*,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, RenderLayers, ViewTarget},
    Render, RenderApp, RenderSet,
};
use bevy_sprite::{Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dViewBindGroup};
use bevy_utils::FloatOrd;

pub struct LineGizmo2dPlugin;

impl Plugin for LineGizmo2dPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent2d, DrawLineGizmo2d>()
            .add_render_command::<Transparent2d, DrawLineJoinGizmo2d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .init_resource::<SpecializedRenderPipelines<LineJoinGizmoPipeline>>()
            .configure_sets(
                Render,
                GizmoRenderSystem::QueueLineGizmos2d.in_set(RenderSet::Queue),
            )
            .add_systems(
                Render,
                (queue_line_gizmos_2d, queue_line_join_gizmos_2d)
                    .in_set(GizmoRenderSystem::QueueLineGizmos2d)
                    .after(prepare_assets::<LineGizmo>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LineGizmoPipeline>();
        render_app.init_resource::<LineJoinGizmoPipeline>();
    }
}

#[derive(Clone, Resource)]
struct LineGizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        LineGizmoPipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineGizmoPipelineKey {
    mesh_key: Mesh2dPipelineKey,
    strip: bool,
}

impl SpecializedRenderPipeline for LineGizmoPipeline {
    type Key = LineGizmoPipelineKey;

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
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline 2D".into()),
            push_constant_ranges: vec![],
        }
    }
}

#[derive(Clone, Resource)]
struct LineJoinGizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineJoinGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        LineJoinGizmoPipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineJoinGizmoPipelineKey {
    mesh_key: Mesh2dPipelineKey,
    joins: GizmoLineJoins,
}

impl SpecializedRenderPipeline for LineJoinGizmoPipeline {
    type Key = LineJoinGizmoPipelineKey;

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

        let entry_point = match key.joins {
            GizmoLineJoins::Miter => "vertex_miter",
            GizmoLineJoins::Round(_) => "vertex_round",
            GizmoLineJoins::Bevel => "vertex_bevel",
        };

        println!("Specializing joins pipeline");
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_JOINS_SHADER_HANDLE,
                entry_point: entry_point.into(),
                shader_defs: shader_defs.clone(),
                buffers: line_join_gizmo_vertex_buffer_layouts(),
            },
            fragment: Some(FragmentState {
                shader: LINE_JOINS_SHADER_HANDLE,
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
            depth_stencil: None,
            multisample: MultisampleState {
                count: key.mesh_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineJoinGizmo Pipeline 2D".into()),
            push_constant_ranges: vec![],
        }
    }
}

type DrawLineGizmo2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo,
);
type DrawLineJoinGizmo2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineJoinGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_line_gizmos_2d(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<Transparent2d>,
        Option<&RenderLayers>,
    )>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo2d>().unwrap();

    for (view, mut transparent_phase, render_layers) in &mut views {
        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        for (entity, handle, config) in &line_gizmos {
            let render_layers = render_layers.copied().unwrap_or_default();
            if !config.render_layers.intersects(&render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(handle) else {
                continue;
            };

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineGizmoPipelineKey {
                    mesh_key,
                    strip: line_gizmo.strip,
                },
            );

            transparent_phase.add(Transparent2d {
                entity,
                draw_function,
                pipeline,
                sort_key: FloatOrd(f32::INFINITY),
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_line_join_gizmos_2d(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<LineJoinGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineJoinGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(
        &ExtractedView,
        &mut RenderPhase<Transparent2d>,
        Option<&RenderLayers>,
    )>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawLineJoinGizmo2d>()
        .unwrap();

    for (view, mut transparent_phase, render_layers) in &mut views {
        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        for (entity, handle, config) in &line_gizmos {
            let render_layers = render_layers.copied().unwrap_or_default();
            if !config.render_layers.intersects(&render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(handle) else {
                continue;
            };

            if !line_gizmo.strip {
                continue;
            }

            let Some(joins) = line_gizmo.joins else {
                continue;
            };

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineJoinGizmoPipelineKey { mesh_key, joins },
            );
            transparent_phase.add(Transparent2d {
                entity,
                draw_function,
                pipeline,
                sort_key: FloatOrd(f32::INFINITY),
                batch_range: 0..1,
                dynamic_offset: None,
            });
        }
    }
}
