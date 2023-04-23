use crate::{
    line_gizmo_vertex_buffer_layouts, DrawLineGizmo, LineGizmo, LineGizmoUniform,
    LINE_SHADER_HANDLE,
};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_core_pipeline::core_2d::Transparent2d;

use bevy_ecs::{
    prelude::Entity,
    query::ROQueryItem,
    schedule::IntoSystemConfigs,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Query, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex},
    render_asset::RenderAssets,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
        RenderPhase, SetItemPipeline, TrackedRenderPass,
    },
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget},
    Render, RenderApp, RenderSet,
};
use bevy_sprite::{Mesh2dPipeline, Mesh2dPipelineKey, SetMesh2dViewBindGroup};
use bevy_utils::FloatOrd;

pub struct LineGizmo2dPlugin;

impl Plugin for LineGizmo2dPlugin {
    fn build(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        render_app
            .add_render_command::<Transparent2d, DrawLineGizmo2d>()
            .init_resource::<LineGizmoPipeline>()
            .init_resource::<SpecializedRenderPipelines<LineGizmoPipeline>>()
            .add_systems(
                Render,
                (queue_line_gizmo_bind_group, queue_line_gizmos_2d).in_set(RenderSet::Queue),
            );
    }
}

#[derive(Clone, Resource)]
struct LineGizmoPipeline {
    mesh_pipeline: Mesh2dPipeline,
    layout: BindGroupLayout,
}

impl FromWorld for LineGizmoPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(LineGizmoUniform::min_size()),
                },
                count: None,
            }],
            label: Some("LineGizmoUniform layout"),
        });

        LineGizmoPipeline {
            mesh_pipeline: render_world.resource::<Mesh2dPipeline>().clone(),
            layout,
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

        let layout = vec![self.mesh_pipeline.view_layout.clone(), self.layout.clone()];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: LINE_SHADER_HANDLE.typed(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: line_gizmo_vertex_buffer_layouts(key.strip),
            },
            fragment: Some(FragmentState {
                shader: LINE_SHADER_HANDLE.typed(),
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

#[derive(Resource)]
struct LineGizmoBindGroup {
    value: BindGroup,
}

fn queue_line_gizmo_bind_group(
    mut commands: Commands,
    line_gizmo_pipeline: Res<LineGizmoPipeline>,
    render_device: Res<RenderDevice>,
    line_gizmo_uniforms: Res<ComponentUniforms<LineGizmoUniform>>,
) {
    if let Some(binding) = line_gizmo_uniforms.uniforms().binding() {
        commands.insert_resource(LineGizmoBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("LineGizmo bindgroup"),
                layout: &line_gizmo_pipeline.layout,
            }),
        });
    }
}

struct SetLineGizmoBindGroup<const I: usize>;
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetLineGizmoBindGroup<I> {
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<DynamicUniformIndex<LineGizmoUniform>>;
    type Param = SRes<LineGizmoBindGroup>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        uniform_index: ROQueryItem<'w, Self::ItemWorldQuery>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(I, &bind_group.into_inner().value, &[uniform_index.index()]);
        RenderCommandResult::Success
    }
}

type DrawLineGizmo2d = (
    SetItemPipeline,
    SetMesh2dViewBindGroup<0>,
    SetLineGizmoBindGroup<1>,
    DrawLineGizmo,
);

#[allow(clippy::too_many_arguments)]
fn queue_line_gizmos_2d(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    pipeline: Res<LineGizmoPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LineGizmoPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    msaa: Res<Msaa>,
    line_gizmos: Query<(Entity, &Handle<LineGizmo>)>,
    line_gizmo_assets: Res<RenderAssets<LineGizmo>>,
    mut views: Query<(&ExtractedView, &mut RenderPhase<Transparent2d>)>,
) {
    let draw_function = draw_functions.read().get_id::<DrawLineGizmo2d>().unwrap();

    for (view, mut transparent_phase) in &mut views {
        let mesh_key = Mesh2dPipelineKey::from_msaa_samples(msaa.samples())
            | Mesh2dPipelineKey::from_hdr(view.hdr);

        for (entity, handle) in &line_gizmos {
            let Some(line_gizmo) = line_gizmo_assets.get(handle) else { continue };

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
                sort_key: FloatOrd(0.),
                batch_range: None,
            });
        }
    }
}
