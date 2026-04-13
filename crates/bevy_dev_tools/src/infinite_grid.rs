//! This module implements an infinite grid with colored major axis.
//!
//! The rendering is not actually infinite and fades out over a customizable distance to avoid
//! artifacts. This fade out is relative to the camera.

use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::{
    prelude::*,
    visibility::{self, NoFrustumCulling, VisibilityClass},
};
use bevy_color::{Color, ColorToComponents};
use bevy_core_pipeline::{
    core_3d::{Transparent3d, TransparentSortingInfo3d},
    FullscreenShader,
};
use bevy_ecs::{
    prelude::*,
    query::ROQueryItem,
    system::{
        lifetimeless::{Read, SRes},
        SystemParamItem,
    },
};
use bevy_math::{Mat3, Vec3, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::ExtractedCamera,
    prelude::*,
    render_phase::{
        AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
        RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
    },
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, BlendState, ColorTargetState, ColorWrites, CompareFunction,
        DepthStencilState, DynamicUniformBuffer, FragmentState, MultisampleState, PipelineCache,
        PrimitiveState, RenderPipelineDescriptor, ShaderStages, ShaderType,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
    },
    renderer::{RenderDevice, RenderQueue},
    sync_world::{RenderEntity, SyncToRenderWorld},
    view::{ExtractedView, RenderVisibleEntities, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, Render, RenderApp, RenderSystems,
};
use bevy_shader::Shader;
use bevy_transform::components::{GlobalTransform, Transform};

/// The plugin required to make the infinite grid work
pub struct InfiniteGridPlugin;

impl Plugin for InfiniteGridPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "infinite_grid.wgsl");
        app.register_type::<InfiniteGrid>()
            .register_type::<InfiniteGridSettings>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<InfiniteGridUniforms>()
            .init_resource::<InfiniteGridDisplaySettingsUniforms>()
            .init_resource::<InfiniteGridPipeline>()
            .init_resource::<SpecializedRenderPipelines<InfiniteGridPipeline>>()
            .add_render_command::<Transparent3d, DrawInfiniteGrid>()
            .add_systems(ExtractSchedule, extract_infinite_grids)
            .add_systems(
                Render,
                prepare_infinite_grids.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Render,
                (
                    prepare_bind_groups_for_infinite_grids,
                    prepare_view_bind_groups,
                )
                    .in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(Render, queue_infinite_grids.in_set(RenderSystems::Queue));
    }
}

/// The component used to represent an infinite grid.
///
/// This is intended for use as a ground plane in editor-like tools.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
#[require(
    InfiniteGridSettings,
    Transform,
    Visibility,
    VisibilityClass,
    NoFrustumCulling,
    SyncToRenderWorld
)]
#[component(on_add = visibility::add_visibility_class::<InfiniteGrid>)]
pub struct InfiniteGrid;

/// Component to configure the infinite grid
///
/// This component can be applied directly on the grid entity or on a camera that can see the grid
#[derive(Component, Copy, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct InfiniteGridSettings {
    /// The color of the X axis
    pub x_axis_color: Color,
    /// The color of the Z axis
    pub z_axis_color: Color,
    /// The color of the minor lines of the grid
    pub minor_line_color: Color,
    /// The color of the major lines of the grid. Every 10th line is considered major
    pub major_line_color: Color,
    /// How far the grid will be visible relative to the camera
    pub fadeout_distance: f32,
    /// How quickly the grid will fadeout
    pub dot_fadeout_strength: f32,
    /// The scale of the distance between the lines. A smaller value increases the distance between
    /// the lines
    pub scale: f32,
}

impl Default for InfiniteGridSettings {
    fn default() -> Self {
        Self {
            // These colors are copied from bevy_feathers but we don't need to depend on it just
            // for that
            x_axis_color: Color::oklcha(0.5232, 0.1404, 13.84, 1.0),
            z_axis_color: Color::oklcha(0.4847, 0.1249, 253.08, 1.0),
            minor_line_color: Color::srgb(0.2, 0.2, 0.2),
            major_line_color: Color::srgb(0.25, 0.25, 0.25),
            fadeout_distance: 100.,
            dot_fadeout_strength: 0.25,
            scale: 1.0,
        }
    }
}

#[derive(Debug, ShaderType)]
struct InfiniteGridUniform {
    rot_matrix: Mat3,
    offset: Vec3,
    normal: Vec3,
}

#[derive(Debug, ShaderType)]
struct InfiniteGridSettingsUniform {
    scale: f32,
    // 1 / fadeout_distance
    one_over_fadeout_distance: f32,
    // 1 / dot_fadeout_strength
    one_over_dot_fadeout: f32,
    x_axis_color: Vec3,
    z_axis_color: Vec3,
    minor_line_color: Vec4,
    major_line_color: Vec4,
}

impl InfiniteGridSettingsUniform {
    fn from_settings(settings: &InfiniteGridSettings) -> Self {
        Self {
            scale: settings.scale,
            one_over_fadeout_distance: 1. / settings.fadeout_distance,
            one_over_dot_fadeout: 1. / settings.dot_fadeout_strength,
            x_axis_color: settings.x_axis_color.to_linear().to_vec3(),
            z_axis_color: settings.z_axis_color.to_linear().to_vec3(),
            minor_line_color: settings.minor_line_color.to_linear().to_vec4(),
            major_line_color: settings.major_line_color.to_linear().to_vec4(),
        }
    }
}

#[derive(Resource, Default)]
struct InfiniteGridUniforms {
    uniforms: DynamicUniformBuffer<InfiniteGridUniform>,
}

#[derive(Resource, Default)]
struct InfiniteGridDisplaySettingsUniforms {
    uniforms: DynamicUniformBuffer<InfiniteGridSettingsUniform>,
}

#[derive(Component)]
struct InfiniteGridUniformOffsets {
    position_offset: u32,
    settings_offset: u32,
}

#[derive(Component)]
struct PerCameraSettingsUniformOffset {
    offset: u32,
}

#[derive(Resource)]
struct InfiniteGridBindGroup {
    value: BindGroup,
}

#[derive(Component)]
struct ViewBindGroup {
    value: BindGroup,
}

struct DrawInfiniteGridCommand;

impl<P: PhaseItem> RenderCommand<P> for DrawInfiniteGridCommand {
    type Param = SRes<InfiniteGridBindGroup>;
    type ViewQuery = (
        Read<ViewUniformOffset>,
        Read<ViewBindGroup>,
        Option<Read<PerCameraSettingsUniformOffset>>,
    );
    type ItemQuery = Read<InfiniteGridUniformOffsets>;

    #[inline]
    fn render<'w>(
        _item: &P,
        (view_uniform, view_bind_group, camera_settings_offset): ROQueryItem<
            'w,
            '_,
            Self::ViewQuery,
        >,
        maybe_base_offsets: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(base_offsets) = maybe_base_offsets else {
            bevy_log::warn!("InfiniteGridUniformOffsets missing");
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(0, &view_bind_group.value, &[view_uniform.offset]);
        pass.set_bind_group(
            1,
            &bind_group.into_inner().value,
            &[
                base_offsets.position_offset,
                camera_settings_offset
                    .map(|cs| cs.offset)
                    .unwrap_or(base_offsets.settings_offset),
            ],
        );
        pass.draw(0..3, 0..1);
        RenderCommandResult::Success
    }
}

type DrawInfiniteGrid = (SetItemPipeline, DrawInfiniteGridCommand);

fn prepare_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    view_uniforms: Res<ViewUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    pipeline_cache: Res<PipelineCache>,
    views: Query<Entity, With<ViewUniformOffset>>,
) {
    let Some(binding) = view_uniforms.uniforms.binding() else {
        return;
    };
    for entity in views.iter() {
        let bind_group = render_device.create_bind_group(
            "infinite_grid_view_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.view_layout),
            &BindGroupEntries::single(binding.clone()),
        );
        commands
            .entity(entity)
            .insert(ViewBindGroup { value: bind_group });
    }
}

fn extract_infinite_grids(
    mut commands: Commands,
    grids: Extract<Query<(RenderEntity, &InfiniteGridSettings, &GlobalTransform)>>,
) {
    let extracted: Vec<_> = grids
        .iter()
        .map(|(entity, grid, transform)| (entity, (*grid, *transform)))
        .collect();
    commands.try_insert_batch(extracted);
}

fn prepare_infinite_grids(
    mut commands: Commands,
    grids: Query<(Entity, &GlobalTransform, &InfiniteGridSettings)>,
    cameras: Query<(Entity, &InfiniteGridSettings), With<ExtractedView>>,
    mut position_uniforms: ResMut<InfiniteGridUniforms>,
    mut settings_uniforms: ResMut<InfiniteGridDisplaySettingsUniforms>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    position_uniforms.uniforms.clear();
    settings_uniforms.uniforms.clear();
    for (entity, transform, settings) in &grids {
        let t = transform.compute_transform();
        let offset = transform.translation();
        let normal = transform.up();
        let rot_matrix = Mat3::from_quat(t.rotation.inverse());
        commands.entity(entity).insert(InfiniteGridUniformOffsets {
            position_offset: position_uniforms.uniforms.push(&InfiniteGridUniform {
                rot_matrix,
                offset,
                normal: *normal,
            }),
            settings_offset: settings_uniforms
                .uniforms
                .push(&InfiniteGridSettingsUniform::from_settings(settings)),
        });
    }

    for (entity, settings) in &cameras {
        commands
            .entity(entity)
            .insert(PerCameraSettingsUniformOffset {
                offset: settings_uniforms
                    .uniforms
                    .push(&InfiniteGridSettingsUniform::from_settings(settings)),
            });
    }

    position_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);

    settings_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

fn prepare_bind_groups_for_infinite_grids(
    mut commands: Commands,
    infinite_grid_uniforms: Res<InfiniteGridUniforms>,
    settings_uniforms: Res<InfiniteGridDisplaySettingsUniforms>,
    pipeline: Res<InfiniteGridPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    let Some((infinite_grid_uniform_binding, settings_binding)) = infinite_grid_uniforms
        .uniforms
        .binding()
        .zip(settings_uniforms.uniforms.binding())
    else {
        return;
    };

    let bind_group = render_device.create_bind_group(
        "infinite_grid_bind_group",
        &pipeline_cache.get_bind_group_layout(&pipeline.infinite_grid_layout),
        &BindGroupEntries::sequential((
            infinite_grid_uniform_binding.clone(),
            settings_binding.clone(),
        )),
    );
    commands.insert_resource(InfiniteGridBindGroup { value: bind_group });
}

fn queue_infinite_grids(
    pipeline_cache: Res<PipelineCache>,
    transparent_draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<InfiniteGridPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<InfiniteGridPipeline>>,
    infinite_grids: Query<&GlobalTransform, With<InfiniteGridSettings>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    mut views: Query<(&ExtractedView, &RenderVisibleEntities, &Msaa), With<ExtractedCamera>>,
) {
    let Some(draw_function_id) = transparent_draw_functions
        .read()
        .get_id::<DrawInfiniteGrid>()
    else {
        bevy_log::warn!("Failed to get DrawInfiniteGrid draw_function_id");
        return;
    };

    for (view, entities, msaa) in views.iter_mut() {
        let Some(phase) = transparent_render_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            GridPipelineKey {
                target_format: view.target_format,
                sample_count: msaa.samples(),
            },
        );

        let Some(render_visible_mesh_entities) = entities.get::<InfiniteGrid>() else {
            continue;
        };
        for (render_entity, main_entity) in render_visible_mesh_entities.iter_visible() {
            let Ok(transform) = infinite_grids.get(*render_entity) else {
                continue;
            };
            // Don't render if the view is directly on the plane
            if !plane_check(transform, view.world_from_view.translation()) {
                continue;
            }
            phase.add(Transparent3d {
                pipeline: pipeline_id,
                entity: (*render_entity, *main_entity),
                draw_function: draw_function_id,
                distance: f32::NEG_INFINITY,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: false,
                sorting_info: TransparentSortingInfo3d::Sorted {
                    mesh_center: Vec3::ZERO,
                    depth_bias: 0.0,
                },
            });
        }
    }
}

/// Checks if the point is one the plane
fn plane_check(plane: &GlobalTransform, point: Vec3) -> bool {
    plane.up().dot(plane.translation() - point).abs() > f32::EPSILON
}

#[derive(Resource)]
struct InfiniteGridPipeline {
    view_layout: BindGroupLayoutDescriptor,
    infinite_grid_layout: BindGroupLayoutDescriptor,
    shader: Handle<Shader>,
    fullscreen_shader: FullscreenShader,
}

impl FromWorld for InfiniteGridPipeline {
    fn from_world(world: &mut World) -> Self {
        let view_layout = BindGroupLayoutDescriptor::new(
            "infinite_grid_view_bind_group_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );
        let infinite_grid_layout = BindGroupLayoutDescriptor::new(
            "infinite_grid_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<InfiniteGridUniform>(true),
                    uniform_buffer::<InfiniteGridSettingsUniform>(true),
                ),
            ),
        );
        let shader = load_embedded_asset!(world.resource::<AssetServer>(), "infinite_grid.wgsl");
        let fullscreen_shader = world.resource::<FullscreenShader>().clone();

        Self {
            view_layout,
            infinite_grid_layout,
            shader,
            fullscreen_shader,
        }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
struct GridPipelineKey {
    target_format: TextureFormat,
    sample_count: u32,
}

impl SpecializedRenderPipeline for InfiniteGridPipeline {
    type Key = GridPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("infinite_grid_render_pipeline".into()),
            layout: vec![self.view_layout.clone(), self.infinite_grid_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            primitive: PrimitiveState {
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Greater),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: MultisampleState {
                count: key.sample_count,
                ..Default::default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}
