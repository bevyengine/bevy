#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! This crate renders `bevy_gizmos` with `bevy_render`.

/// System set label for the systems handling the rendering of gizmos.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GizmoRenderSystems {
    /// Adds gizmos to the [`Transparent2d`](bevy_core_pipeline::core_2d::Transparent2d) render phase
    #[cfg(feature = "bevy_sprite_render")]
    QueueLineGizmos2d,
    /// Adds gizmos to the [`Transparent3d`](bevy_core_pipeline::core_3d::Transparent3d) render phase
    #[cfg(feature = "bevy_pbr")]
    QueueLineGizmos3d,
}

pub mod retained;

#[cfg(feature = "bevy_sprite_render")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use bevy_app::{App, Plugin};
use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::Res,
};

use {bevy_gizmos::config::GizmoMeshConfig, bevy_mesh::VertexBufferLayout};

use {
    crate::retained::extract_linegizmos,
    bevy_asset::AssetId,
    bevy_ecs::{
        component::Component,
        entity::Entity,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            Commands, SystemParamItem,
        },
    },
    bevy_math::{Affine3, Affine3A, Vec4},
    bevy_render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayoutEntries,
            Buffer, BufferInitDescriptor, BufferUsages, ShaderStages, ShaderType, VertexFormat,
        },
        renderer::RenderDevice,
        sync_world::{MainEntity, TemporaryRenderEntity},
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    },
    bytemuck::cast_slice,
};

use bevy_render::render_resource::{
    BindGroupLayoutDescriptor, PipelineCache, VertexAttribute, VertexStepMode,
};

use bevy_gizmos::{
    config::{GizmoConfigStore, GizmoLineJoint},
    GizmoAsset, GizmoHandles,
};

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
///
/// Requires to be loaded after [`PbrPlugin`](bevy_pbr::PbrPlugin) or [`SpriteRenderPlugin`](bevy_sprite_render::SpriteRenderPlugin).
#[derive(Default)]
pub struct GizmoRenderPlugin;

impl Plugin for GizmoRenderPlugin {
    fn build(&self, app: &mut App) {
        {
            use bevy_asset::embedded_asset;
            embedded_asset!(app, "lines.wgsl");
            embedded_asset!(app, "line_joints.wgsl");
        }

        app.add_plugins(UniformComponentPlugin::<LineGizmoUniform>::default())
            .add_plugins(RenderAssetPlugin::<GpuLineGizmo>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(RenderStartup, init_line_gizmo_uniform_bind_group_layout);

            render_app.add_systems(
                Render,
                prepare_line_gizmo_bind_group.in_set(RenderSystems::PrepareBindGroups),
            );

            render_app.add_systems(ExtractSchedule, (extract_gizmo_data, extract_linegizmos));

            #[cfg(feature = "bevy_sprite_render")]
            if app.is_plugin_added::<bevy_sprite_render::SpriteRenderPlugin>() {
                app.add_plugins(pipeline_2d::LineGizmo2dPlugin);
            } else {
                tracing::warn!("bevy_sprite_render feature is enabled but bevy_sprite_render::SpriteRenderPlugin was not detected. Are you sure you loaded GizmoPlugin after SpriteRenderPlugin?");
            }
            #[cfg(feature = "bevy_pbr")]
            if app.is_plugin_added::<bevy_pbr::PbrPlugin>() {
                app.add_plugins(pipeline_3d::LineGizmo3dPlugin);
            } else {
                tracing::warn!("bevy_pbr feature is enabled but bevy_pbr::PbrPlugin was not detected. Are you sure you loaded GizmoPlugin after PbrPlugin?");
            }
        } else {
            tracing::warn!("bevy_render feature is enabled but RenderApp was not detected. Are you sure you loaded GizmoPlugin after RenderPlugin?");
        }
    }
}

fn init_line_gizmo_uniform_bind_group_layout(mut commands: Commands) {
    let line_layout = BindGroupLayoutDescriptor::new(
        "LineGizmoUniform layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<LineGizmoUniform>(true),
        ),
    );

    commands.insert_resource(LineGizmoUniformBindgroupLayout {
        layout: line_layout,
    });
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<GizmoHandles>>,
    config: Extract<Res<GizmoConfigStore>>,
) {
    use bevy_gizmos::config::GizmoLineStyle;
    use bevy_utils::once;
    use tracing::warn;

    for (group_type_id, handle) in handles.handles() {
        let Some((config, _)) = config.get_config_dyn(group_type_id) else {
            continue;
        };

        if !config.enabled {
            continue;
        }

        #[cfg_attr(
            not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
            expect(
                unused_variables,
                reason = "`handle` is unused when bevy_pbr and bevy_sprite_render are both disabled."
            )
        )]
        let Some(handle) = handle
        else {
            continue;
        };

        let joints_resolution = if let GizmoLineJoint::Round(resolution) = config.line.joints {
            resolution
        } else {
            0
        };

        let (gap_scale, line_scale) = if let GizmoLineStyle::Dashed {
            gap_scale,
            line_scale,
        } = config.line.style
        {
            if gap_scale <= 0.0 {
                once!(warn!("When using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the gap scale should be greater than zero."));
            }
            if line_scale <= 0.0 {
                once!(warn!("When using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the line scale should be greater than zero."));
            }
            (gap_scale, line_scale)
        } else {
            (1.0, 1.0)
        };

        commands.spawn((
            LineGizmoUniform {
                world_from_local: Affine3::from(&Affine3A::IDENTITY).to_transpose(),
                line_width: config.line.width,
                depth_bias: config.depth_bias,
                joints_resolution,
                gap_scale,
                line_scale,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite_render"))]
            GizmoMeshConfig {
                line_perspective: config.line.perspective,
                line_style: config.line.style,
                line_joints: config.line.joints,
                render_layers: config.render_layers.clone(),
                handle: handle.clone(),
            },
            // The immediate mode API does not have a main world entity to refer to,
            // but we do need MainEntity on this render entity for the systems to find it.
            MainEntity::from(Entity::PLACEHOLDER),
            TemporaryRenderEntity,
        ));
    }
}

#[derive(Component, ShaderType, Clone, Copy)]
struct LineGizmoUniform {
    world_from_local: [Vec4; 3],
    line_width: f32,
    depth_bias: f32,
    // Only used by gizmo line t if the current configs `line_joints` is set to `GizmoLineJoint::Round(_)`
    joints_resolution: u32,
    // Only used if the current configs `line_style` is set to `GizmoLineStyle::Dashed{_}`
    gap_scale: f32,
    line_scale: f32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding: bevy_math::Vec3,
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "fields are unused when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
#[derive(Debug, Clone)]
struct GpuLineGizmo {
    list_position_buffer: Buffer,
    list_color_buffer: Buffer,
    list_vertex_count: u32,
    strip_position_buffer: Buffer,
    strip_color_buffer: Buffer,
    strip_vertex_count: u32,
}

impl RenderAsset for GpuLineGizmo {
    type SourceAsset = GizmoAsset;
    type Param = SRes<RenderDevice>;

    fn prepare_asset(
        gizmo: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        render_device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let list_position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: cast_slice(&gizmo.buffer().list_positions),
        });

        let list_color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: cast_slice(&gizmo.buffer().list_colors),
        });

        let strip_position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Strip Position Buffer"),
            contents: cast_slice(&gizmo.buffer().strip_positions),
        });

        let strip_color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Strip Color Buffer"),
            contents: cast_slice(&gizmo.buffer().strip_colors),
        });

        Ok(GpuLineGizmo {
            list_position_buffer,
            list_color_buffer,
            list_vertex_count: gizmo.buffer().list_positions.len() as u32,
            strip_position_buffer,
            strip_color_buffer,
            strip_vertex_count: gizmo.buffer().strip_positions.len() as u32,
        })
    }
}

#[derive(Resource)]
struct LineGizmoUniformBindgroupLayout {
    layout: BindGroupLayoutDescriptor,
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "fields are unused when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
#[derive(Resource)]
struct LineGizmoUniformBindgroup {
    bindgroup: BindGroup,
}

fn prepare_line_gizmo_bind_group(
    mut commands: Commands,
    line_gizmo_uniform_layout: Res<LineGizmoUniformBindgroupLayout>,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    line_gizmo_uniforms: Res<ComponentUniforms<LineGizmoUniform>>,
) {
    if let Some(binding) = line_gizmo_uniforms.uniforms().binding() {
        commands.insert_resource(LineGizmoUniformBindgroup {
            bindgroup: render_device.create_bind_group(
                "LineGizmoUniform bindgroup",
                &pipeline_cache.get_bind_group_layout(&line_gizmo_uniform_layout.layout),
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "struct is not constructed when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
struct SetLineGizmoBindGroup<const I: usize>;

impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetLineGizmoBindGroup<I> {
    type Param = SRes<LineGizmoUniformBindgroup>;
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<LineGizmoUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        uniform_index: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(uniform_index) = uniform_index else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(
            I,
            &bind_group.into_inner().bindgroup,
            &[uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "struct is not constructed when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
struct DrawLineGizmo<const STRIP: bool>;

impl<P: PhaseItem, const STRIP: bool> RenderCommand<P> for DrawLineGizmo<STRIP> {
    type Param = SRes<RenderAssets<GpuLineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<GizmoMeshConfig>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        let vertex_count = if STRIP {
            line_gizmo.strip_vertex_count
        } else {
            line_gizmo.list_vertex_count
        };

        if vertex_count < 2 {
            return RenderCommandResult::Success;
        }

        let instances = if STRIP {
            let item_size = VertexFormat::Float32x3.size();
            let buffer_size = line_gizmo.strip_position_buffer.size() - item_size;

            pass.set_vertex_buffer(0, line_gizmo.strip_position_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(1, line_gizmo.strip_position_buffer.slice(item_size..));

            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.strip_color_buffer.size() - item_size;

            pass.set_vertex_buffer(2, line_gizmo.strip_color_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(3, line_gizmo.strip_color_buffer.slice(item_size..));

            vertex_count - 1
        } else {
            pass.set_vertex_buffer(0, line_gizmo.list_position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.list_color_buffer.slice(..));

            vertex_count / 2
        };

        pass.draw(0..6, 0..instances);

        RenderCommandResult::Success
    }
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "struct is not constructed when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
struct DrawLineJointGizmo;

impl<P: PhaseItem> RenderCommand<P> for DrawLineJointGizmo {
    type Param = SRes<RenderAssets<GpuLineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<GizmoMeshConfig>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        if line_gizmo.strip_vertex_count <= 2 {
            return RenderCommandResult::Success;
        };

        if config.line_joints == GizmoLineJoint::None {
            return RenderCommandResult::Success;
        };

        let instances = {
            let item_size = VertexFormat::Float32x3.size();
            // position_a
            let buffer_size_a = line_gizmo.strip_position_buffer.size() - item_size * 2;
            pass.set_vertex_buffer(0, line_gizmo.strip_position_buffer.slice(..buffer_size_a));
            // position_b
            let buffer_size_b = line_gizmo.strip_position_buffer.size() - item_size;
            pass.set_vertex_buffer(
                1,
                line_gizmo
                    .strip_position_buffer
                    .slice(item_size..buffer_size_b),
            );
            // position_c
            pass.set_vertex_buffer(2, line_gizmo.strip_position_buffer.slice(item_size * 2..));

            // color
            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.strip_color_buffer.size() - item_size;
            // This corresponds to the color of position_b, hence starts from `item_size`
            pass.set_vertex_buffer(
                3,
                line_gizmo.strip_color_buffer.slice(item_size..buffer_size),
            );

            line_gizmo.strip_vertex_count - 2
        };

        let vertices = match config.line_joints {
            GizmoLineJoint::None => unreachable!(),
            GizmoLineJoint::Miter => 6,
            GizmoLineJoint::Round(resolution) => resolution * 3,
            GizmoLineJoint::Bevel => 3,
        };

        pass.draw(0..vertices, 0..instances);

        RenderCommandResult::Success
    }
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "function is unused when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
fn line_gizmo_vertex_buffer_layouts(strip: bool) -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let mut position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let mut color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 2,
        }],
    };

    if strip {
        vec![
            position_layout.clone(),
            {
                position_layout.attributes[0].shader_location = 1;
                position_layout
            },
            color_layout.clone(),
            {
                color_layout.attributes[0].shader_location = 3;
                color_layout
            },
        ]
    } else {
        position_layout.array_stride *= 2;
        position_layout.attributes.push(VertexAttribute {
            format: Float32x3,
            offset: Float32x3.size(),
            shader_location: 1,
        });

        color_layout.array_stride *= 2;
        color_layout.attributes.push(VertexAttribute {
            format: Float32x4,
            offset: Float32x4.size(),
            shader_location: 3,
        });

        vec![position_layout, color_layout]
    }
}

#[cfg_attr(
    not(any(feature = "bevy_pbr", feature = "bevy_sprite_render")),
    expect(
        dead_code,
        reason = "function is unused when bevy_pbr and bevy_sprite_render are both disabled."
    )
)]
fn line_joint_gizmo_vertex_buffer_layouts() -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let mut position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 3,
        }],
    };

    vec![
        position_layout.clone(),
        {
            position_layout.attributes[0].shader_location = 1;
            position_layout.clone()
        },
        {
            position_layout.attributes[0].shader_location = 2;
            position_layout
        },
        color_layout.clone(),
    ]
}
