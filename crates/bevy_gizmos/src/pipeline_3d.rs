use crate::{
    config::{GizmoLineJoint, GizmoLineStyle, GizmoMeshConfig},
    init_line_gizmo_uniform_bind_group_layout, line_gizmo_vertex_buffer_layouts,
    line_joint_gizmo_vertex_buffer_layouts, DrawLineGizmo, DrawLineJointGizmo, GizmoRenderSystems,
    GpuLineGizmo, LineGizmoUniformBindgroupLayout, SetLineGizmoBindGroup,
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_camera::visibility::RenderLayers;
use bevy_core_pipeline::{
    core_3d::{Transparent3d, CORE_3D_DEPTH_FORMAT},
    oit::OrderIndependentTransparencySettings,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
};

use bevy_ecs::{
    prelude::Entity,
    query::Has,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::BevyDefault as _;
use bevy_pbr::{MeshPipeline, MeshPipelineKey, SetMeshViewBindGroup};
use bevy_render::{
    render_asset::{prepare_assets, RenderAssets},
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItemExtraIndex, SetItemPipeline,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    view::{ExtractedView, Msaa, ViewTarget},
    Render, RenderApp, RenderSystems,
};
use bevy_render::{sync_world::MainEntity, RenderStartup};
use bevy_shader::Shader;
use bevy_utils::default;
use tracing::error;

pub struct LineGizmo3dPlugin;
impl Plugin for LineGizmo3dPlugin {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_command::<Transparent3d, DrawLineGizmo3d>()
            .add_render_command::<Transparent3d, DrawLineGizmo3dStrip>()
            .add_render_command::<Transparent3d, DrawLineJointGizmo3d>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .init_resource::<SpecializedRenderPipelines<LineJointGizmoPipeline>>()
            .configure_sets(
                Render,
                GizmoRenderSystems::QueueLineGizmos3d.in_set(RenderSystems::Queue),
            )
            .add_systems(
                RenderStartup,
                init_line_gizmo_pipelines.after(init_line_gizmo_uniform_bind_group_layout),
            )
            .add_systems(
                Render,
                (queue_line_gizmos_3d, queue_line_joint_gizmos_3d)
                    .in_set(GizmoRenderSystems::QueueLineGizmos3d)
                    .after(prepare_assets::<GpuLineGizmo>),
            );
    }
}

#[derive(Clone, Resource)]
struct LineGizmoPipeline {
    mesh_pipeline: MeshPipeline,
    uniform_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

fn init_line_gizmo_pipelines(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    uniform_bind_group_layout: Res<LineGizmoUniformBindgroupLayout>,
    asset_server: Res<AssetServer>,
) {
    commands.insert_resource(LineGizmoPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        uniform_layout: uniform_bind_group_layout.layout.clone(),
        shader: load_embedded_asset!(asset_server.as_ref(), "lines.wgsl"),
    });
    commands.insert_resource(LineJointGizmoPipeline {
        mesh_pipeline: mesh_pipeline.clone(),
        uniform_layout: uniform_bind_group_layout.layout.clone(),
        shader: load_embedded_asset!(asset_server.as_ref(), "line_joints.wgsl"),
    });
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineGizmoPipelineKey {
    view_key: MeshPipelineKey,
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

        let format = if key.view_key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let view_layout = self
            .mesh_pipeline
            .get_view_layout(key.view_key.into())
            .clone();
        let layout = vec![view_layout.main_layout.clone(), self.uniform_layout.clone()];

        let fragment_entry_point = match key.line_style {
            GizmoLineStyle::Solid => "fragment_solid",
            GizmoLineStyle::Dotted => "fragment_dotted",
            GizmoLineStyle::Dashed { .. } => "fragment_dashed",
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: line_gizmo_vertex_buffer_layouts(key.strip),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some(fragment_entry_point.into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout,
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.view_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineGizmo 3d Pipeline".into()),
            ..default()
        }
    }
}

#[derive(Clone, Resource)]
struct LineJointGizmoPipeline {
    mesh_pipeline: MeshPipeline,
    uniform_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct LineJointGizmoPipelineKey {
    view_key: MeshPipelineKey,
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

        let format = if key.view_key.contains(MeshPipelineKey::HDR) {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        let view_layout = self
            .mesh_pipeline
            .get_view_layout(key.view_key.into())
            .clone();
        let layout = vec![view_layout.main_layout.clone(), self.uniform_layout.clone()];

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
                shader: self.shader.clone(),
                entry_point: Some(entry_point.into()),
                shader_defs: shader_defs.clone(),
                buffers: line_joint_gizmo_vertex_buffer_layouts(),
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout,
            depth_stencil: Some(DepthStencilState {
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: key.view_key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("LineJointGizmo 3d Pipeline".into()),
            ..default()
        }
    }
}

type DrawLineGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo<false>,
);
type DrawLineGizmo3dStrip = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo<true>,
);
type DrawLineJointGizmo3d = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineJointGizmo,
);

fn queue_line_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    line_gizmos: Query<(Entity, &MainEntity, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<GpuLineGizmo>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(
        &ExtractedView,
        &Msaa,
        Option<&RenderLayers>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
            Has<OrderIndependentTransparencySettings>,
        ),
    )>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo3d>().unwrap();
    let draw_function_strip = draw_functions
        .read()
        .get_id::<DrawLineGizmo3dStrip>()
        .unwrap();

    for (
        view,
        msaa,
        render_layers,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass, oit),
    ) in &views
    {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let render_layers = render_layers.unwrap_or_default();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        if oit {
            view_key |= MeshPipelineKey::OIT_ENABLED;
        }

        for (entity, main_entity, config) in &line_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(&config.handle) else {
                continue;
            };

            if line_gizmo.list_vertex_count > 0 {
                let pipeline = pipelines.specialize(
                    &pipeline_cache,
                    &pipeline,
                    LineGizmoPipelineKey {
                        view_key,
                        strip: false,
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
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: true,
                });
            }

            if line_gizmo.strip_vertex_count >= 2 {
                let pipeline = pipelines.specialize(
                    &pipeline_cache,
                    &pipeline,
                    LineGizmoPipelineKey {
                        view_key,
                        strip: true,
                        perspective: config.line_perspective,
                        line_style: config.line_style,
                    },
                );
                transparent_phase.add(Transparent3d {
                    entity: (entity, *main_entity),
                    draw_function: draw_function_strip,
                    pipeline,
                    distance: 0.,
                    batch_range: 0..1,
                    extra_index: PhaseItemExtraIndex::None,
                    indexed: true,
                });
            }
        }
    }
}

fn queue_line_joint_gizmos_3d(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<LineJointGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineJointGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    line_gizmos: Query<(Entity, &MainEntity, &GizmoMeshConfig)>,
    line_gizmo_assets: Res<RenderAssets<GpuLineGizmo>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(
        &ExtractedView,
        &Msaa,
        Option<&RenderLayers>,
        (
            Has<NormalPrepass>,
            Has<DepthPrepass>,
            Has<MotionVectorPrepass>,
            Has<DeferredPrepass>,
        ),
    )>,
) {
    let draw_function = draw_functions
        .read()
        .get_id::<DrawLineJointGizmo3d>()
        .unwrap();

    for (
        view,
        msaa,
        render_layers,
        (normal_prepass, depth_prepass, motion_vector_prepass, deferred_prepass),
    ) in &views
    {
        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let render_layers = render_layers.unwrap_or_default();

        let mut view_key = MeshPipelineKey::from_msaa_samples(msaa.samples())
            | MeshPipelineKey::from_hdr(view.hdr);

        if normal_prepass {
            view_key |= MeshPipelineKey::NORMAL_PREPASS;
        }

        if depth_prepass {
            view_key |= MeshPipelineKey::DEPTH_PREPASS;
        }

        if motion_vector_prepass {
            view_key |= MeshPipelineKey::MOTION_VECTOR_PREPASS;
        }

        if deferred_prepass {
            view_key |= MeshPipelineKey::DEFERRED_PREPASS;
        }

        for (entity, main_entity, config) in &line_gizmos {
            if !config.render_layers.intersects(render_layers) {
                continue;
            }

            let Some(line_gizmo) = line_gizmo_assets.get(&config.handle) else {
                continue;
            };

            if line_gizmo.strip_vertex_count < 3 || config.line_joints == GizmoLineJoint::None {
                continue;
            }

            let pipeline = pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                LineJointGizmoPipelineKey {
                    view_key,
                    perspective: config.line_perspective,
                    joints: config.line_joints,
                },
            );

            transparent_phase.add(Transparent3d {
                entity: (entity, *main_entity),
                draw_function,
                pipeline,
                distance: 0.,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}
