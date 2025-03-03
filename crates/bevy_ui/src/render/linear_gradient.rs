use core::{hash::Hash, ops::Range};

use crate::*;
use bevy_asset::*;
use bevy_color::{ColorToComponents, LinearRgba};
use bevy_ecs::{
    prelude::Component,
    system::{
        lifetimeless::{Read, SRes},
        *,
    },
};
use bevy_image::prelude::*;
use bevy_math::{FloatOrd, Mat4, Rect, Vec2, Vec3Swizzles, Vec4Swizzles};
use bevy_render::sync_world::MainEntity;
use bevy_render::{
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    sync_world::TemporaryRenderEntity,
    view::*,
    Extract, ExtractSchedule, Render, RenderSet,
};
use bevy_sprite::BorderRect;
use bevy_transform::prelude::GlobalTransform;
use bytemuck::{Pod, Zeroable};

pub const UI_LINEAR_GRADIENT_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("10cd61e3-bbf7-47fa-91c8-16cbe806378c");

pub struct LinearGradientPlugin;

impl Plugin for LinearGradientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, compute_color_stops.in_set(UiSystem::PostLayout));

        load_internal_asset!(
            app,
            UI_LINEAR_GRADIENT_SHADER_HANDLE,
            "linear_gradient.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawLinearGradientFns>()
                .init_resource::<ExtractedLinearGradients>()
                .init_resource::<LinearGradientMeta>()
                .init_resource::<SpecializedRenderPipelines<LinearGradientPipeline>>()
                .add_systems(
                    ExtractSchedule,
                    extract_linear_gradients.in_set(RenderUiSystem::ExtractLinearGradient),
                )
                .add_systems(
                    Render,
                    (
                        queue_linear_gradient.in_set(RenderSet::Queue),
                        prepare_linear_gradient.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<LinearGradientPipeline>();
        }
    }
}

#[derive(Component)]
pub struct LinearGradientBatch {
    pub range: Range<u32>,
}

#[derive(Resource)]
pub struct LinearGradientMeta {
    vertices: RawBufferVec<UiGradientVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for LinearGradientMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct LinearGradientPipeline {
    pub view_layout: BindGroupLayout,
}

impl FromWorld for LinearGradientPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            "ui_linear_gradient_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        LinearGradientPipeline { view_layout }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiTextureSlicePipelineKey {
    pub hdr: bool,
}

impl SpecializedRenderPipeline for LinearGradientPipeline {
    type Key = UiTextureSlicePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // flags
                VertexFormat::Uint32,
                // radius
                VertexFormat::Float32x4,
                // border
                VertexFormat::Float32x4,
                // size
                VertexFormat::Float32x2,
                // point
                VertexFormat::Float32x2,
                // start_point
                VertexFormat::Float32x2,
                // dir
                VertexFormat::Float32x2,
                // start_color
                VertexFormat::Float32x4,
                // start_len
                VertexFormat::Float32,
                // end_len
                VertexFormat::Float32,
                // end color
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: UI_LINEAR_GRADIENT_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: UI_LINEAR_GRADIENT_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone()],
            push_constant_ranges: Vec::new(),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("ui_linear_gradient_pipeline".into()),
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub struct ExtractedLinearGradient {
    pub stack_index: u32,
    pub transform: Mat4,
    pub rect: Rect,
    pub clip: Option<Rect>,
    pub extracted_camera_entity: Entity,
    pub g_start: Vec2,
    pub g_dir: Vec2,
    pub start: f32,
    pub start_color: LinearRgba,
    pub end: f32,
    pub end_color: LinearRgba,
    pub node_type: NodeType,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    border_radius: ResolvedBorderRadius,
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    border: BorderRect,
}

#[derive(Resource, Default)]
pub struct ExtractedLinearGradients {
    pub items: Vec<ExtractedLinearGradient>,
}

pub fn extract_linear_gradients(
    mut commands: Commands,
    mut extracted_linear_gradients: ResMut<ExtractedLinearGradients>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    gradients_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedNodeTarget,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &LinearGradient,
            &ComputedColorStops,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();
    //println!("extract gradients");

    for (entity, uinode, target, transform, inherited_visibility, clip, linear_gradient, stops) in
        &gradients_query
    {
        // Skip invisible images
        if !inherited_visibility.get() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target) else {
            continue;
        };

        if stops.0.len() == 1 {
            // With a single color stop there's no gradient, fill the node with the color
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                stack_index: uinode.stack_index,
                color: stops.0[0].0.into(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.size,
                },
                image: AssetId::default(),
                clip: clip.map(|clip| clip.clip),
                extracted_camera_entity,
                item: ExtractedUiItem::Node {
                    atlas_scaling: None,
                    flip_x: false,
                    flip_y: false,
                    border_radius: uinode.border_radius,
                    border: uinode.border,
                    node_type: NodeType::Rect,
                    transform: transform.compute_matrix(),
                },
                main_entity: entity.into(),
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
            });
            continue;
        }

        for stop_pair in stops.0.windows(2) {
            let start = stop_pair[0];
            let end = stop_pair[1];
            extracted_linear_gradients
                .items
                .push(ExtractedLinearGradient {
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    stack_index: uinode.stack_index,
                    transform: transform.compute_matrix(),
                    start: start.1,
                    g_start: Vec2::ZERO,
                    start_color: start.0.into(),
                    g_dir: Vec2::Y,
                    end: end.1,
                    end_color: end.0.into(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: uinode.size,
                    },
                    clip: clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    main_entity: entity.into(),
                    node_type: NodeType::Rect,
                    border_radius: uinode.border_radius,
                    border: uinode.border,
                });
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "it's a system that needs a lot of them"
)]
pub fn queue_linear_gradient(
    extracted_gradients: ResMut<ExtractedLinearGradients>,
    linear_gradients_pipeline: Res<LinearGradientPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<LinearGradientPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut render_views: Query<&UiCameraView, With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawLinearGradientFns>();
    for (index, gradient) in extracted_gradients.items.iter().enumerate() {
        let Ok(default_camera_view) = render_views.get_mut(gradient.extracted_camera_entity) else {
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
            &linear_gradients_pipeline,
            UiTextureSlicePipelineKey { hdr: view.hdr },
        );

        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: (gradient.render_entity, gradient.main_entity),
            sort_key: (
                FloatOrd(gradient.stack_index as f32 + stack_z_offsets::NODE),
                gradient.render_entity.index(),
            ),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            index,
            indexed: true,
        });
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiGradientVertex {
    position: [f32; 3],
    uv: [f32; 2],
    flags: u32,
    radius: [f32; 4],
    border: [f32; 4],
    size: [f32; 2],
    point: [f32; 2],
    g_start: [f32; 2],
    g_dir: [f32; 2],
    start_color: [f32; 4],
    start_len: f32,
    end_len: f32,
    end_color: [f32; 4],
}

pub fn prepare_linear_gradient(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<LinearGradientMeta>,
    mut extracted_gradients: ResMut<ExtractedLinearGradients>,
    view_uniforms: Res<ViewUniforms>,
    linear_gradients_pipeline: Res<LinearGradientPipeline>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, LinearGradientBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "linear_gradient_view_bind_group",
            &linear_gradients_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(gradient) = extracted_gradients
                    .items
                    .get(item.index)
                    .filter(|n| item.entity() == n.render_entity)
                {
                    *item.batch_range_mut() = item_index as u32..item_index as u32 + 1;
                    //println!("batch range = {:?}", item.batch_range());
                    let uinode_rect = gradient.rect;

                    let rect_size = uinode_rect.size().extend(1.0);

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS
                        .map(|pos| (gradient.transform * (pos * rect_size).extend(1.)).xyz());
                    let points = QUAD_VERTEX_POSITIONS.map(|pos| pos.xy() * rect_size.xy());

                    // Calculate the effect of clipping
                    // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
                    let positions_diff = if let Some(clip) = gradient.clip {
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

                    let points = [
                        points[0] + positions_diff[0],
                        points[1] + positions_diff[1],
                        points[2] + positions_diff[2],
                        points[3] + positions_diff[3],
                    ];

                    let transformed_rect_size = gradient.transform.transform_vector3(rect_size);

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if gradient.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
                    }

                    let uvs = { [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y] };

                    let start_color = gradient.start_color.to_f32_array();
                    let end_color = gradient.end_color.to_f32_array();

                    let flags = if gradient.node_type == NodeType::Border {
                        shader_flags::BORDER
                    } else {
                        0
                    };
                    
                    for i in 0..4 {
                        ui_meta.vertices.push(UiGradientVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            flags: flags | shader_flags::CORNERS[i],
                            radius: [
                                gradient.border_radius.top_left,
                                gradient.border_radius.top_right,
                                gradient.border_radius.bottom_right,
                                gradient.border_radius.bottom_left,
                            ],
                            border: [
                                gradient.border.left,
                                gradient.border.top,
                                gradient.border.right,
                                gradient.border.bottom,
                            ],
                            size: rect_size.xy().into(),
                            g_start: gradient.g_start.into(),
                            g_dir: gradient.g_dir.into(),
                            point: points[i].into(),
                            start_color,
                            start_len: gradient.start,
                            end_len: gradient.end,
                            end_color,
                        });
                    }

                    for &i in &QUAD_INDICES {
                        ui_meta.indices.push(indices_index + i as u32);
                    }

                    batches.push((
                        item.entity(),
                        LinearGradientBatch {
                            range: vertices_index..vertices_index + 6,
                        },
                    ));

                    vertices_index += 6;
                    indices_index += 4;
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
    extracted_gradients.items.clear();
}

pub type DrawLinearGradientFns = (
    SetItemPipeline,
    SetLinearGradientViewBindGroup<0>,
    DrawLinearGradient,
);

pub struct SetLinearGradientViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetLinearGradientViewBindGroup<I> {
    type Param = SRes<LinearGradientMeta>;
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

pub struct DrawLinearGradient;
impl<P: PhaseItem> RenderCommand<P> for DrawLinearGradient {
    type Param = SRes<LinearGradientMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<LinearGradientBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w LinearGradientBatch>,
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
