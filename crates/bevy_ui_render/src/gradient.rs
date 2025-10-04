use core::{
    f32::consts::{FRAC_PI_2, TAU},
    hash::Hash,
    ops::Range,
};

use super::shader_flags::BORDER_ALL;
use crate::*;
use bevy_asset::*;
use bevy_color::{ColorToComponents, Hsla, Hsva, LinearRgba, Oklaba, Oklcha, Srgba};
use bevy_ecs::{
    prelude::Component,
    system::{
        lifetimeless::{Read, SRes},
        *,
    },
};
use bevy_image::prelude::*;
use bevy_math::{
    ops::{cos, sin},
    FloatOrd, Rect, Vec2,
};
use bevy_math::{Affine2, Vec2Swizzles};
use bevy_mesh::VertexBufferLayout;
use bevy_render::{
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    sync_world::TemporaryRenderEntity,
    view::*,
    Extract, ExtractSchedule, Render, RenderSystems,
};
use bevy_render::{sync_world::MainEntity, RenderStartup};
use bevy_shader::Shader;
use bevy_sprite::BorderRect;
use bevy_ui::{
    BackgroundGradient, BorderGradient, ColorStop, ComputedUiRenderTargetInfo, ConicGradient,
    Gradient, InterpolationColorSpace, LinearGradient, RadialGradient, ResolvedBorderRadius, Val,
};
use bevy_utils::default;
use bytemuck::{Pod, Zeroable};

pub struct GradientPlugin;

impl Plugin for GradientPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "gradient.wgsl");

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawGradientFns>()
                .init_resource::<ExtractedGradients>()
                .init_resource::<ExtractedColorStops>()
                .init_resource::<GradientMeta>()
                .init_resource::<SpecializedRenderPipelines<GradientPipeline>>()
                .add_systems(RenderStartup, init_gradient_pipeline)
                .add_systems(
                    ExtractSchedule,
                    extract_gradients
                        .in_set(RenderUiSystems::ExtractGradient)
                        .after(extract_uinode_background_colors),
                )
                .add_systems(
                    Render,
                    (
                        queue_gradient.in_set(RenderSystems::Queue),
                        prepare_gradient.in_set(RenderSystems::PrepareBindGroups),
                    ),
                );
        }
    }
}

#[derive(Component)]
pub struct GradientBatch {
    pub range: Range<u32>,
}

#[derive(Resource)]
pub struct GradientMeta {
    vertices: RawBufferVec<UiGradientVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for GradientMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct GradientPipeline {
    pub view_layout: BindGroupLayout,
    pub shader: Handle<Shader>,
}

pub fn init_gradient_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    let view_layout = render_device.create_bind_group_layout(
        "ui_gradient_view_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<ViewUniform>(true),
        ),
    );

    commands.insert_resource(GradientPipeline {
        view_layout,
        shader: load_embedded_asset!(asset_server.as_ref(), "gradient.wgsl"),
    });
}

pub fn compute_gradient_line_length(angle: f32, size: Vec2) -> f32 {
    let center = 0.5 * size;
    let v = Vec2::new(sin(angle), -cos(angle));

    let (pos_corner, neg_corner) = if v.x >= 0.0 && v.y <= 0.0 {
        (size.with_y(0.), size.with_x(0.))
    } else if v.x >= 0.0 && v.y > 0.0 {
        (size, Vec2::ZERO)
    } else if v.x < 0.0 && v.y <= 0.0 {
        (Vec2::ZERO, size)
    } else {
        (size.with_x(0.), size.with_y(0.))
    };

    let t_pos = (pos_corner - center).dot(v);
    let t_neg = (neg_corner - center).dot(v);

    (t_pos - t_neg).abs()
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiGradientPipelineKey {
    anti_alias: bool,
    color_space: InterpolationColorSpace,
    pub hdr: bool,
}

impl SpecializedRenderPipeline for GradientPipeline {
    type Key = UiGradientPipelineKey;

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
                // hint
                VertexFormat::Float32,
            ],
        );
        let color_space = match key.color_space {
            InterpolationColorSpace::Oklaba => "IN_OKLAB",
            InterpolationColorSpace::Oklcha => "IN_OKLCH",
            InterpolationColorSpace::OklchaLong => "IN_OKLCH_LONG",
            InterpolationColorSpace::Srgba => "IN_SRGB",
            InterpolationColorSpace::LinearRgba => "IN_LINEAR_RGB",
            InterpolationColorSpace::Hsla => "IN_HSL",
            InterpolationColorSpace::HslaLong => "IN_HSL_LONG",
            InterpolationColorSpace::Hsva => "IN_HSV",
            InterpolationColorSpace::HsvaLong => "IN_HSV_LONG",
        };

        let shader_defs = if key.anti_alias {
            vec![color_space.into(), "ANTI_ALIAS".into()]
        } else {
            vec![color_space.into()]
        };

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
            label: Some("ui_gradient_pipeline".into()),
            ..default()
        }
    }
}

pub enum ResolvedGradient {
    Linear { angle: f32 },
    Conic { center: Vec2, start: f32 },
    Radial { center: Vec2, size: Vec2 },
}

pub struct ExtractedGradient {
    pub stack_index: u32,
    pub transform: Affine2,
    pub rect: Rect,
    pub clip: Option<Rect>,
    pub extracted_camera_entity: Entity,
    /// range into `ExtractedColorStops`
    pub stops_range: Range<usize>,
    pub node_type: NodeType,
    pub main_entity: MainEntity,
    pub render_entity: Entity,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub border_radius: ResolvedBorderRadius,
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: BorderRect,
    pub resolved_gradient: ResolvedGradient,
    pub color_space: InterpolationColorSpace,
}

#[derive(Resource, Default)]
pub struct ExtractedGradients {
    pub items: Vec<ExtractedGradient>,
}

#[derive(Resource, Default)]
pub struct ExtractedColorStops(pub Vec<(LinearRgba, f32, f32)>);

// Interpolate implicit stops (where position is `f32::NAN`)
// If the first and last stops are implicit set them to the `min` and `max` values
// so that we always have explicit start and end points to interpolate between.
fn interpolate_color_stops(stops: &mut [(LinearRgba, f32, f32)], min: f32, max: f32) {
    if stops[0].1.is_nan() {
        stops[0].1 = min;
    }
    if stops.last().unwrap().1.is_nan() {
        stops.last_mut().unwrap().1 = max;
    }

    let mut i = 1;

    while i < stops.len() - 1 {
        let point = stops[i].1;
        if point.is_nan() {
            let start = i;
            let mut end = i + 1;
            while end < stops.len() - 1 && stops[end].1.is_nan() {
                end += 1;
            }
            let start_point = stops[start - 1].1;
            let end_point = stops[end].1;
            let steps = end - start;
            let step = (end_point - start_point) / (steps + 1) as f32;
            for j in 0..steps {
                stops[i + j].1 = start_point + step * (j + 1) as f32;
            }
            i = end;
        }
        i += 1;
    }
}

fn compute_color_stops(
    stops: &[ColorStop],
    scale_factor: f32,
    length: f32,
    target_size: Vec2,
    scratch: &mut Vec<(LinearRgba, f32, f32)>,
    extracted_color_stops: &mut Vec<(LinearRgba, f32, f32)>,
) {
    // resolve the physical distances of explicit stops and sort them
    scratch.extend(stops.iter().filter_map(|stop| {
        stop.point
            .resolve(scale_factor, length, target_size)
            .ok()
            .map(|physical_point| (stop.color.to_linear(), physical_point, stop.hint))
    }));
    scratch.sort_by_key(|(_, point, _)| FloatOrd(*point));

    let min = scratch
        .first()
        .map(|(_, min, _)| *min)
        .unwrap_or(0.)
        .min(0.);

    // get the position of the last explicit stop and use the full length of the gradient if no explicit stops
    let max = scratch
        .last()
        .map(|(_, max, _)| *max)
        .unwrap_or(length)
        .max(length);

    let mut sorted_stops_drain = scratch.drain(..);

    let range_start = extracted_color_stops.len();

    // Fill the extracted color stops buffer
    extracted_color_stops.extend(stops.iter().map(|stop| {
        if stop.point == Val::Auto {
            (stop.color.to_linear(), f32::NAN, stop.hint)
        } else {
            sorted_stops_drain.next().unwrap()
        }
    }));

    interpolate_color_stops(&mut extracted_color_stops[range_start..], min, max);
}

pub fn extract_gradients(
    mut commands: Commands,
    mut extracted_gradients: ResMut<ExtractedGradients>,
    mut extracted_color_stops: ResMut<ExtractedColorStops>,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    gradients_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedUiTargetCamera,
            &ComputedUiRenderTargetInfo,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            AnyOf<(&BackgroundGradient, &BorderGradient)>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();
    let mut sorted_stops = vec![];

    for (
        entity,
        uinode,
        camera,
        target,
        transform,
        inherited_visibility,
        clip,
        (gradient, gradient_border),
    ) in &gradients_query
    {
        // Skip invisible images
        if !inherited_visibility.get() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        for (gradients, node_type) in [
            (gradient.map(|g| &g.0), NodeType::Rect),
            (gradient_border.map(|g| &g.0), NodeType::Border(BORDER_ALL)),
        ]
        .iter()
        .filter_map(|(g, n)| g.map(|g| (g, *n)))
        {
            for gradient in gradients.iter() {
                if gradient.is_empty() {
                    continue;
                }
                if let Some(color) = gradient.get_single() {
                    // With a single color stop there's no gradient, fill the node with the color
                    extracted_uinodes.uinodes.push(ExtractedUiNode {
                        z_order: uinode.stack_index as f32
                            + match node_type {
                                NodeType::Rect => stack_z_offsets::GRADIENT,
                                NodeType::Border(_) => stack_z_offsets::BORDER_GRADIENT,
                            },
                        image: AssetId::default(),
                        clip: clip.map(|clip| clip.clip),
                        extracted_camera_entity,
                        transform: transform.into(),
                        item: ExtractedUiItem::Node {
                            color: color.into(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border_radius: uinode.border_radius,
                            border: uinode.border,
                            node_type,
                        },
                        main_entity: entity.into(),
                        render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    });
                    continue;
                }
                match gradient {
                    Gradient::Linear(LinearGradient {
                        color_space,
                        angle,
                        stops,
                    }) => {
                        let length = compute_gradient_line_length(*angle, uinode.size);

                        let range_start = extracted_color_stops.0.len();

                        compute_color_stops(
                            stops,
                            target.scale_factor(),
                            length,
                            target.physical_size().as_vec2(),
                            &mut sorted_stops,
                            &mut extracted_color_stops.0,
                        );

                        extracted_gradients.items.push(ExtractedGradient {
                            render_entity: commands.spawn(TemporaryRenderEntity).id(),
                            stack_index: uinode.stack_index,
                            transform: transform.into(),
                            stops_range: range_start..extracted_color_stops.0.len(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            clip: clip.map(|clip| clip.clip),
                            extracted_camera_entity,
                            main_entity: entity.into(),
                            node_type,
                            border_radius: uinode.border_radius,
                            border: uinode.border,
                            resolved_gradient: ResolvedGradient::Linear { angle: *angle },
                            color_space: *color_space,
                        });
                    }
                    Gradient::Radial(RadialGradient {
                        color_space,
                        position: center,
                        shape,
                        stops,
                    }) => {
                        let c = center.resolve(
                            target.scale_factor(),
                            uinode.size,
                            target.physical_size().as_vec2(),
                        );

                        let size = shape.resolve(
                            c,
                            target.scale_factor(),
                            uinode.size,
                            target.physical_size().as_vec2(),
                        );

                        let length = size.x;

                        let range_start = extracted_color_stops.0.len();
                        compute_color_stops(
                            stops,
                            target.scale_factor(),
                            length,
                            target.physical_size().as_vec2(),
                            &mut sorted_stops,
                            &mut extracted_color_stops.0,
                        );

                        extracted_gradients.items.push(ExtractedGradient {
                            render_entity: commands.spawn(TemporaryRenderEntity).id(),
                            stack_index: uinode.stack_index,
                            transform: transform.into(),
                            stops_range: range_start..extracted_color_stops.0.len(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            clip: clip.map(|clip| clip.clip),
                            extracted_camera_entity,
                            main_entity: entity.into(),
                            node_type,
                            border_radius: uinode.border_radius,
                            border: uinode.border,
                            resolved_gradient: ResolvedGradient::Radial { center: c, size },
                            color_space: *color_space,
                        });
                    }
                    Gradient::Conic(ConicGradient {
                        color_space,
                        start,
                        position: center,
                        stops,
                    }) => {
                        let g_start = center.resolve(
                            target.scale_factor(),
                            uinode.size,
                            target.physical_size().as_vec2(),
                        );
                        let range_start = extracted_color_stops.0.len();

                        // sort the explicit stops
                        sorted_stops.extend(stops.iter().filter_map(|stop| {
                            stop.angle.map(|angle| {
                                (stop.color.to_linear(), angle.clamp(0., TAU), stop.hint)
                            })
                        }));
                        sorted_stops.sort_by_key(|(_, angle, _)| FloatOrd(*angle));
                        let mut sorted_stops_drain = sorted_stops.drain(..);

                        // fill the extracted stops buffer
                        extracted_color_stops.0.extend(stops.iter().map(|stop| {
                            if stop.angle.is_none() {
                                (stop.color.to_linear(), f32::NAN, stop.hint)
                            } else {
                                sorted_stops_drain.next().unwrap()
                            }
                        }));

                        interpolate_color_stops(
                            &mut extracted_color_stops.0[range_start..],
                            0.,
                            TAU,
                        );

                        extracted_gradients.items.push(ExtractedGradient {
                            render_entity: commands.spawn(TemporaryRenderEntity).id(),
                            stack_index: uinode.stack_index,
                            transform: transform.into(),
                            stops_range: range_start..extracted_color_stops.0.len(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            clip: clip.map(|clip| clip.clip),
                            extracted_camera_entity,
                            main_entity: entity.into(),
                            node_type,
                            border_radius: uinode.border_radius,
                            border: uinode.border,
                            resolved_gradient: ResolvedGradient::Conic {
                                start: *start,
                                center: g_start,
                            },
                            color_space: *color_space,
                        });
                    }
                }
            }
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "it's a system that needs a lot of them"
)]
pub fn queue_gradient(
    extracted_gradients: ResMut<ExtractedGradients>,
    gradients_pipeline: Res<GradientPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<GradientPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut render_views: Query<(&UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawGradientFns>();
    for (index, gradient) in extracted_gradients.items.iter().enumerate() {
        let Ok((default_camera_view, ui_anti_alias)) =
            render_views.get_mut(gradient.extracted_camera_entity)
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
            &gradients_pipeline,
            UiGradientPipelineKey {
                anti_alias: matches!(ui_anti_alias, None | Some(UiAntiAlias::On)),
                color_space: gradient.color_space,
                hdr: view.hdr,
            },
        );

        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: (gradient.render_entity, gradient.main_entity),
            sort_key: FloatOrd(
                gradient.stack_index as f32
                    + match gradient.node_type {
                        NodeType::Rect => stack_z_offsets::GRADIENT,
                        NodeType::Border(_) => stack_z_offsets::BORDER_GRADIENT,
                    },
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
    hint: f32,
}

fn convert_color_to_space(color: LinearRgba, space: InterpolationColorSpace) -> [f32; 4] {
    match space {
        InterpolationColorSpace::Oklaba => {
            let oklaba: Oklaba = color.into();
            [oklaba.lightness, oklaba.a, oklaba.b, oklaba.alpha]
        }
        InterpolationColorSpace::Oklcha | InterpolationColorSpace::OklchaLong => {
            let oklcha: Oklcha = color.into();
            [
                oklcha.lightness,
                oklcha.chroma,
                // The shader expects normalized hues
                oklcha.hue / 360.,
                oklcha.alpha,
            ]
        }
        InterpolationColorSpace::Srgba => {
            let srgba: Srgba = color.into();
            [srgba.red, srgba.green, srgba.blue, srgba.alpha]
        }
        InterpolationColorSpace::LinearRgba => color.to_f32_array(),
        InterpolationColorSpace::Hsla | InterpolationColorSpace::HslaLong => {
            let hsla: Hsla = color.into();
            // The shader expects normalized hues
            [hsla.hue / 360., hsla.saturation, hsla.lightness, hsla.alpha]
        }
        InterpolationColorSpace::Hsva | InterpolationColorSpace::HsvaLong => {
            let hsva: Hsva = color.into();
            // The shader expects normalized hues
            [hsva.hue / 360., hsva.saturation, hsva.value, hsva.alpha]
        }
    }
}

pub fn prepare_gradient(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<GradientMeta>,
    mut extracted_gradients: ResMut<ExtractedGradients>,
    mut extracted_color_stops: ResMut<ExtractedColorStops>,
    view_uniforms: Res<ViewUniforms>,
    gradients_pipeline: Res<GradientPipeline>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut previous_len: Local<usize>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, GradientBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "gradient_view_bind_group",
            &gradients_pipeline.view_layout,
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
                    let uinode_rect = gradient.rect;

                    let rect_size = uinode_rect.size();

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        gradient
                            .transform
                            .transform_point2(pos * rect_size)
                            .extend(0.)
                    });
                    let corner_points = QUAD_VERTEX_POSITIONS.map(|pos| pos * rect_size);

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
                        corner_points[0] + positions_diff[0],
                        corner_points[1] + positions_diff[1],
                        corner_points[2] + positions_diff[2],
                        corner_points[3] + positions_diff[3],
                    ];

                    let transformed_rect_size = gradient.transform.transform_vector2(rect_size);

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

                    let mut flags = if let NodeType::Border(borders) = gradient.node_type {
                        borders
                    } else {
                        0
                    };

                    let (g_start, g_dir, g_flags) = match gradient.resolved_gradient {
                        ResolvedGradient::Linear { angle } => {
                            let corner_index = (angle - FRAC_PI_2).rem_euclid(TAU) / FRAC_PI_2;
                            (
                                corner_points[corner_index as usize].into(),
                                // CSS angles increase in a clockwise direction
                                [sin(angle), -cos(angle)],
                                0,
                            )
                        }
                        ResolvedGradient::Conic { center, start } => {
                            (center.into(), [start, 0.], shader_flags::CONIC)
                        }
                        ResolvedGradient::Radial { center, size } => (
                            center.into(),
                            Vec2::splat(if size.y != 0. { size.x / size.y } else { 1. }).into(),
                            shader_flags::RADIAL,
                        ),
                    };

                    flags |= g_flags;

                    let range = gradient.stops_range.start..gradient.stops_range.end - 1;
                    let mut segment_count = 0;

                    for stop_index in range {
                        let mut start_stop = extracted_color_stops.0[stop_index];
                        let end_stop = extracted_color_stops.0[stop_index + 1];
                        if start_stop.1 == end_stop.1 {
                            if stop_index == gradient.stops_range.end - 2 {
                                if 0 < segment_count {
                                    start_stop.0 = LinearRgba::NONE;
                                }
                            } else {
                                continue;
                            }
                        }
                        let start_color =
                            convert_color_to_space(start_stop.0, gradient.color_space);
                        let end_color = convert_color_to_space(end_stop.0, gradient.color_space);
                        let mut stop_flags = flags;
                        if 0. < start_stop.1
                            && (stop_index == gradient.stops_range.start || segment_count == 0)
                        {
                            stop_flags |= shader_flags::FILL_START;
                        }
                        if stop_index == gradient.stops_range.end - 2 {
                            stop_flags |= shader_flags::FILL_END;
                        }

                        for i in 0..4 {
                            ui_meta.vertices.push(UiGradientVertex {
                                position: positions_clipped[i].into(),
                                uv: uvs[i].into(),
                                flags: stop_flags | shader_flags::CORNERS[i],
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
                                g_start,
                                g_dir,
                                point: points[i].into(),
                                start_color,
                                start_len: start_stop.1,
                                end_len: end_stop.1,
                                end_color,
                                hint: start_stop.2,
                            });
                        }

                        for &i in &QUAD_INDICES {
                            ui_meta.indices.push(indices_index + i as u32);
                        }
                        indices_index += 4;
                        segment_count += 1;
                    }

                    if 0 < segment_count {
                        let vertices_count = 6 * segment_count;

                        batches.push((
                            item.entity(),
                            GradientBatch {
                                range: vertices_index..(vertices_index + vertices_count),
                            },
                        ));

                        vertices_index += vertices_count;
                    }
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.try_insert_batch(batches);
    }
    extracted_gradients.items.clear();
    extracted_color_stops.0.clear();
}

pub type DrawGradientFns = (SetItemPipeline, SetGradientViewBindGroup<0>, DrawGradient);

pub struct SetGradientViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetGradientViewBindGroup<I> {
    type Param = SRes<GradientMeta>;
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

pub struct DrawGradient;
impl<P: PhaseItem> RenderCommand<P> for DrawGradient {
    type Param = SRes<GradientMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<GradientBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w GradientBatch>,
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
