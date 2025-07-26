//! Box shadows rendering

use core::{hash::Hash, ops::Range};

use bevy_app::prelude::*;
use bevy_asset::*;
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_ecs::prelude::*;
use bevy_ecs::{
    prelude::Component,
    system::{
        lifetimeless::{Read, SRes},
        *,
    },
};
use bevy_image::BevyDefault as _;
use bevy_math::{vec2, Affine2, FloatOrd, Rect, Vec2};
use bevy_render::sync_world::{MainEntity, TemporaryRenderEntity};
use bevy_render::{
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    view::*,
    Extract, ExtractSchedule, Render, RenderSystems,
};
use bevy_render::{RenderApp, RenderStartup};
use bevy_ui::{
    BoxShadow, CalculatedClip, ComputedNode, ComputedNodeTarget, ResolvedBorderRadius,
    UiGlobalTransform, Val,
};
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};

use crate::{BoxShadowSamples, RenderUiSystems, TransparentUi, UiCameraMap};

use super::{stack_z_offsets, UiCameraView, QUAD_INDICES, QUAD_VERTEX_POSITIONS};

/// A plugin that enables the rendering of box shadows.
pub struct BoxShadowPlugin;

impl Plugin for BoxShadowPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "box_shadow.wgsl");

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawBoxShadows>()
                .init_resource::<ExtractedBoxShadows>()
                .init_resource::<BoxShadowMeta>()
                .init_resource::<SpecializedRenderPipelines<BoxShadowPipeline>>()
                .add_systems(RenderStartup, init_box_shadow_pipeline)
                .add_systems(
                    ExtractSchedule,
                    extract_shadows.in_set(RenderUiSystems::ExtractBoxShadows),
                )
                .add_systems(
                    Render,
                    (
                        queue_shadows.in_set(RenderSystems::Queue),
                        prepare_shadows.in_set(RenderSystems::PrepareBindGroups),
                    ),
                );
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BoxShadowVertex {
    position: [f32; 3],
    uvs: [f32; 2],
    vertex_color: [f32; 4],
    size: [f32; 2],
    radius: [f32; 4],
    blur: f32,
    bounds: [f32; 2],
}

#[derive(Component)]
pub struct UiShadowsBatch {
    pub range: Range<u32>,
    pub camera: Entity,
}

/// Contains the vertices and bind groups to be sent to the GPU
#[derive(Resource)]
pub struct BoxShadowMeta {
    vertices: RawBufferVec<BoxShadowVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for BoxShadowMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct BoxShadowPipeline {
    pub view_layout: BindGroupLayout,
    pub shader: Handle<Shader>,
}

pub fn init_box_shadow_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    let view_layout = render_device.create_bind_group_layout(
        "box_shadow_view_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ViewUniform>(true),
        ),
    );

    commands.insert_resource(BoxShadowPipeline {
        view_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "box_shadow.wgsl"),
    });
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct BoxShadowPipelineKey {
    pub hdr: bool,
    /// Number of samples, a higher value results in better quality shadows.
    pub samples: u32,
}

impl SpecializedRenderPipeline for BoxShadowPipeline {
    type Key = BoxShadowPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
                // target rect size
                VertexFormat::Float32x2,
                // corner radius values (top left, top right, bottom right, bottom left)
                VertexFormat::Float32x4,
                // blur radius
                VertexFormat::Float32,
                // outer size
                VertexFormat::Float32x2,
            ],
        );
        let shader_defs = vec![ShaderDefVal::UInt(
            "SHADOW_SAMPLES".to_string(),
            key.samples,
        )];

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            layout: vec![self.view_layout.clone()],
            label: Some("box_shadow_pipeline".into()),
            ..default()
        }
    }
}

/// Description of a shadow to be sorted and queued for rendering
pub struct ExtractedBoxShadow {
    pub stack_index: u32,
    pub transform: Affine2,
    pub bounds: Vec2,
    pub clip: Option<Rect>,
    pub extracted_camera_entity: Entity,
    pub color: LinearRgba,
    pub radius: ResolvedBorderRadius,
    pub blur_radius: f32,
    pub size: Vec2,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
}

/// List of extracted shadows to be sorted and queued for rendering
#[derive(Resource, Default)]
pub struct ExtractedBoxShadows {
    pub box_shadows: Vec<ExtractedBoxShadow>,
}

pub fn extract_shadows(
    mut commands: Commands,
    mut extracted_box_shadows: ResMut<ExtractedBoxShadows>,
    box_shadow_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            &BoxShadow,
            Option<&CalculatedClip>,
            &ComputedNodeTarget,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut mapping = camera_map.get_mapper();

    for (entity, uinode, transform, visibility, box_shadow, clip, camera) in &box_shadow_query {
        // Skip if no visible shadows
        if !visibility.get() || box_shadow.is_empty() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = mapping.map(camera) else {
            continue;
        };

        let ui_physical_viewport_size = camera.physical_size().as_vec2();
        let scale_factor = uinode.inverse_scale_factor.recip();

        for drop_shadow in box_shadow.iter() {
            if drop_shadow.color.is_fully_transparent() {
                continue;
            }

            let resolve_val = |val, base, scale_factor| match val {
                Val::Auto => 0.,
                Val::Px(px) => px * scale_factor,
                Val::Percent(percent) => percent / 100. * base,
                Val::Vw(percent) => percent / 100. * ui_physical_viewport_size.x,
                Val::Vh(percent) => percent / 100. * ui_physical_viewport_size.y,
                Val::VMin(percent) => percent / 100. * ui_physical_viewport_size.min_element(),
                Val::VMax(percent) => percent / 100. * ui_physical_viewport_size.max_element(),
            };

            let spread_x = resolve_val(drop_shadow.spread_radius, uinode.size().x, scale_factor);
            let spread_ratio = (spread_x + uinode.size().x) / uinode.size().x;

            let spread = vec2(spread_x, uinode.size().y * spread_ratio - uinode.size().y);

            let blur_radius = resolve_val(drop_shadow.blur_radius, uinode.size().x, scale_factor);
            let offset = vec2(
                resolve_val(drop_shadow.x_offset, uinode.size().x, scale_factor),
                resolve_val(drop_shadow.y_offset, uinode.size().y, scale_factor),
            );

            let shadow_size = uinode.size() + spread;
            if shadow_size.cmple(Vec2::ZERO).any() {
                continue;
            }

            let radius = ResolvedBorderRadius {
                top_left: uinode.border_radius.top_left * spread_ratio,
                top_right: uinode.border_radius.top_right * spread_ratio,
                bottom_left: uinode.border_radius.bottom_left * spread_ratio,
                bottom_right: uinode.border_radius.bottom_right * spread_ratio,
            };

            extracted_box_shadows.box_shadows.push(ExtractedBoxShadow {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                stack_index: uinode.stack_index,
                transform: Affine2::from(transform) * Affine2::from_translation(offset),
                color: drop_shadow.color.into(),
                bounds: shadow_size + 6. * blur_radius,
                clip: clip.map(|clip| clip.clip),
                extracted_camera_entity,
                radius,
                blur_radius,
                size: shadow_size,
                main_entity: entity.into(),
            });
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "it's a system that needs a lot of them"
)]
pub fn queue_shadows(
    extracted_box_shadows: ResMut<ExtractedBoxShadows>,
    box_shadow_pipeline: Res<BoxShadowPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BoxShadowPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut render_views: Query<(&UiCameraView, Option<&BoxShadowSamples>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawBoxShadows>();
    for (index, extracted_shadow) in extracted_box_shadows.box_shadows.iter().enumerate() {
        let entity = extracted_shadow.render_entity;
        let Ok((default_camera_view, shadow_samples)) =
            render_views.get_mut(extracted_shadow.extracted_camera_entity)
        else {
            continue;
        };

        let Ok(view) = camera_views.get(default_camera_view.0) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view.retained_view_entity)
        else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &box_shadow_pipeline,
            BoxShadowPipelineKey {
                hdr: view.hdr,
                samples: shadow_samples.copied().unwrap_or_default().0,
            },
        );

        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: (entity, extracted_shadow.main_entity),
            sort_key: FloatOrd(extracted_shadow.stack_index as f32 + stack_z_offsets::BOX_SHADOW),

            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: true,
        });
    }
}

pub fn prepare_shadows(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<BoxShadowMeta>,
    mut extracted_shadows: ResMut<ExtractedBoxShadows>,
    view_uniforms: Res<ViewUniforms>,
    box_shadow_pipeline: Res<BoxShadowPipeline>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiShadowsBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "box_shadow_view_bind_group",
            &box_shadow_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(box_shadow) = extracted_shadows
                    .box_shadows
                    .get(item.index)
                    .filter(|n| item.entity() == n.render_entity)
                {
                    let rect_size = box_shadow.bounds;

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        box_shadow
                            .transform
                            .transform_point2(pos * rect_size)
                            .extend(0.)
                    });

                    // Calculate the effect of clipping
                    // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
                    let positions_diff = if let Some(clip) = box_shadow.clip {
                        [
                            Vec2::new(
                                f32::max(clip.min.x - positions[0].x, 0.),
                                f32::max(clip.min.y - positions[0].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[1].x, 0.),
                                f32::max(clip.min.y - positions[1].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[2].x, 0.),
                                f32::min(clip.max.y - positions[2].y, 0.),
                            ),
                            Vec2::new(
                                f32::max(clip.min.x - positions[3].x, 0.),
                                f32::min(clip.max.y - positions[3].y, 0.),
                            ),
                        ]
                    } else {
                        [Vec2::ZERO; 4]
                    };

                    let positions_clipped = [
                        positions[0] + positions_diff[0].extend(0.),
                        positions[1] + positions_diff[1].extend(0.),
                        positions[2] + positions_diff[2].extend(0.),
                        positions[3] + positions_diff[3].extend(0.),
                    ];

                    let transformed_rect_size = box_shadow.transform.transform_vector2(rect_size);

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if box_shadow.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
                    }

                    let uvs = [
                        Vec2::new(positions_diff[0].x, positions_diff[0].y),
                        Vec2::new(
                            box_shadow.bounds.x + positions_diff[1].x,
                            positions_diff[1].y,
                        ),
                        Vec2::new(
                            box_shadow.bounds.x + positions_diff[2].x,
                            box_shadow.bounds.y + positions_diff[2].y,
                        ),
                        Vec2::new(
                            positions_diff[3].x,
                            box_shadow.bounds.y + positions_diff[3].y,
                        ),
                    ]
                    .map(|pos| pos / box_shadow.bounds);

                    for i in 0..4 {
                        ui_meta.vertices.push(BoxShadowVertex {
                            position: positions_clipped[i].into(),
                            uvs: uvs[i].into(),
                            vertex_color: box_shadow.color.to_f32_array(),
                            size: box_shadow.size.into(),
                            radius: box_shadow.radius.into(),
                            blur: box_shadow.blur_radius,
                            bounds: rect_size.into(),
                        });
                    }

                    for &i in &QUAD_INDICES {
                        ui_meta.indices.push(indices_index + i as u32);
                    }

                    batches.push((
                        item.entity(),
                        UiShadowsBatch {
                            range: vertices_index..vertices_index + 6,
                            camera: box_shadow.extracted_camera_entity,
                        },
                    ));

                    vertices_index += 6;
                    indices_index += 4;

                    // shadows are sent to the gpu non-batched
                    *ui_phase.items[item_index].batch_range_mut() =
                        item_index as u32..item_index as u32 + 1;
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.try_insert_batch(batches);
    }
    extracted_shadows.box_shadows.clear();
}

pub type DrawBoxShadows = (SetItemPipeline, SetBoxShadowViewBindGroup<0>, DrawBoxShadow);

pub struct SetBoxShadowViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetBoxShadowViewBindGroup<I> {
    type Param = SRes<BoxShadowMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(view_bind_group) = ui_meta.into_inner().view_bind_group.as_ref() else {
            return RenderCommandResult::Failure("view_bind_group not available");
        };
        pass.set_bind_group(I, view_bind_group, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}

pub struct DrawBoxShadow;
impl<P: PhaseItem> RenderCommand<P> for DrawBoxShadow {
    type Param = SRes<BoxShadowMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiShadowsBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiShadowsBatch>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };
        let ui_meta = ui_meta.into_inner();
        let Some(vertices) = ui_meta.vertices.buffer() else {
            return RenderCommandResult::Failure("missing vertices to draw ui");
        };
        let Some(indices) = ui_meta.indices.buffer() else {
            return RenderCommandResult::Failure("missing indices to draw ui");
        };

        // Store the vertices
        pass.set_vertex_buffer(0, vertices.slice(..));
        // Define how to "connect" the vertices
        pass.set_index_buffer(indices.slice(..), 0, IndexFormat::Uint32);
        // Draw the vertices
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}
