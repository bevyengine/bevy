use bevy_ecs::schedule::IntoSystemConfigs;
use crate::{
    config::{GizmoLineJoint, GizmoLineStyle, GizmoMeshConfig}, line_gizmo_vertex_buffer_layouts, line_joint_gizmo_vertex_buffer_layouts, view::{prepare_view_bind_groups, OnlyViewLayout, SetViewBindGroup}, DrawLineGizmo, DrawLineJointGizmo, GizmoRenderSystem, GpuLineGizmo, LineGizmoUniformBindgroupLayout, SetLineGizmoBindGroup, LINE_JOINT_SHADER_HANDLE, LINE_SHADER_HANDLE
};
use bevy_app::{App, Plugin};
use bevy_core_pipeline::core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT};

use bevy_ecs::{
    prelude::Entity, system::{Query, Res, ResMut, Resource}, world::{FromWorld, World}
};
use bevy_image::BevyDefault as _;
use bevy_render::sync_world::MainEntity;
use bevy_render::{
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, SetItemPipeline,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    view::{ExtractedView, Msaa, RenderLayers, ViewTarget},
    Render, RenderApp, RenderSet,
};
use bevy_utils::tracing::error;
pub struct LineGizmo3dPlugin;
impl Plugin for LineGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawLineGizmo3d>()
            .add_render_command::<Transparent3d, DrawLineJointGizmo3d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .init_resource::<SpecializedRenderPipelines<LineJointGizmoPipeline>>()
            .add_systems(
                Render,
                (queue_line_gizmos_3d, queue_line_joint_gizmos_3d, prepare_view_bind_groups.in_set(RenderSet::Prepare))
                    .in_set(GizmoRenderSystem::QueueLineGizmos3d)
                    .after(prepare_assets::<GpuLineGizmo>),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<LineGizmoPipeline>();
        render_app.init_resource::<LineJointGizmoPipeline>();
    }
}

#[derive(Clone, Resource)]
pub(crate) struct LineGizmoPipeline {
    view_layout: BindGroupLayout,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let view_layout = render_world.resource::<OnlyViewLayout>().0.clone();

        LineGizmoPipeline {
            view_layout,
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) struct LineGizmoPipelineKey {
    msaa: Msaa,
    hdr: bool,
    strip: bool,
    perspective: bool,
    line_style: GizmoLineStyle,
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

        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let layout = vec![
            self.view_layout.clone(),
            self.uniform_layout.clone()
        ];

        let fragment_entry_point = match key.line_style {
            GizmoLineStyle::Solid => "fragment_solid",
            GizmoLineStyle::Dotted => "fragment_dotted",
        };

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
                entry_point: fragment_entry_point.into(),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo Pipeline".into()),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        }
    }
}

#[derive(Clone, Resource)]
struct LineJointGizmoPipeline {
    view_layout: BindGroupLayout,
    uniform_layout: BindGroupLayout,
}

impl FromWorld for LineJointGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let view_layout = render_world.resource::<OnlyViewLayout>().0.clone();

        LineJointGizmoPipeline {
            view_layout,
            uniform_layout: render_world
                .resource::<LineGizmoUniformBindgroupLayout>()
                .layout
                .clone(),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineJointGizmoPipelineKey {
    msaa: Msaa,
    hdr: bool,
    perspective: bool,
    joints: GizmoLineJoint,
}

impl SpecializedRenderPipeline for LineJointGizmoPipeline {
    type Key = LineJointGizmoPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![
            #[cfg(feature = "webgl")]
            "SIXTEEN_BYTE_ALIGNMENT".into(),
        ];

        if key.perspective {
            shader_defs.push("PERSPECTIVE".into());
        }

        let format = if key.hdr {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        // FIXME: don't store view layout ?

        let layout = vec![
            self.view_layout.clone(),
            self.uniform_layout.clone()
        ];

        if key.joints == GizmoLineJoint::None {
            error!("There is no entry point for line joints with GizmoLineJoints::None. Please consider aborting the drawing process before reaching this stage.");
        };

        let entry_point = match key.joints {
            GizmoLineJoint::Miter => "vertex_miter",
            GizmoLineJoint::Round(_) => "vertex_round",
            GizmoLineJoint::None | GizmoLineJoint::Bevel => "vertex_bevel",
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_JOINT_SHADER_HANDLE,
                entry_point: entry_point.into(),
                shader_defs: shader_defs.clone(),
                buffers: line_joint_gizmo_vertex_buffer_layouts(),
            },
            fragment: Some(FragmentState {
                shader: LINE_JOINT_SHADER_HANDLE,
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
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.msaa.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineJointGizmo Pipeline".into()),
            push_constant_ranges: vec![],
            zero_initialize_workgroup_memory: false,
        }
    }
}

type DrawLineGizmo3d = (
    SetItemPipeline,
    SetViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo,
);
type DrawLineJointGizmo3d = (
    SetItemPipeline,
    SetViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineJointGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_line_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    line_gizmos: Query<(Entity, &MainEntity, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<GpuLineGizmo>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &Msaa,
        Option<&RenderLayers>,
    )>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo3d>().unwrap();

    for (
        view_entity,
        view,
        msaa,
        render_layers,
    ) in &mut views
    {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let render_layers = render_layers.unwrap_or_default();

        for (entity, main_entity, config) in &line_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(&config.handle) else {
                continue;
            };

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineGizmoPipelineKey {
                    msaa: *msaa,
                    hdr: view.hdr,
                    strip: line_gizmo.strip,
                    perspective: config.line_perspective,
                    line_style: config.line_style,
                },
            );

            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_line_joint_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineJointGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineJointGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    line_gizmos: Query<(Entity, &MainEntity, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<GpuLineGizmo>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &Msaa,
        Option<&RenderLayers>,
    )>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawLineJointGizmo3d>()
        .unwrap();

    for (
        view_entity,
        view,
        msaa,
        render_layers,
    ) in &mut views
    {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let render_layers = render_layers.unwrap_or_default();


        for (entity, main_entity, config) in &line_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(&config.handle) else {
                continue;
            };

            if !line_gizmo.strip || line_gizmo.joints == GizmoLineJoint::None {
                continue;
            }

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineJointGizmoPipelineKey {
                    msaa: *msaa,
                    hdr: view.hdr,
                    perspective: config.line_perspective,
                    joints: line_gizmo.joints,
                },
            );

            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::NONE,
            });
        }
    }
}
