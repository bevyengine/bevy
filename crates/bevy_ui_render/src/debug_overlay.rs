use crate::{
    clipping::clip_polygon, extract_layout::ExtractedUiLayout, shader_flags, DrawUi, TransparentUi,
    UiAntiAlias, UiCameraView, UiMeta, UiPipeline, UiPipelineKey, UiVertex, QUAD_UVS,
    QUAD_VERTEX_POSITIONS,
};

use bevy_camera::{Camera2d, Camera3d};
use bevy_color::{ColorToComponents, Hsla, LinearRgba};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::{Entity, EntityHashMap, EntityHashSet},
    lifecycle::RemovedComponents,
    prelude::*,
    query::Changed,
    resource::Resource,
};

use bevy_math::{Affine2, FloatOrd, Rect, Vec2};

use bevy_reflect::Reflect;
use bevy_render::{
    render_phase::{DrawFunctions, PhaseItemExtraIndex, ViewSortedRenderPhases},
    render_resource::{PipelineCache, SpecializedRenderPipelines},
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet, RenderEntity},
    view::ExtractedView,
    Extract,
};

use bevy_ui::{CalculatedClip, ResolvedBorderRadius, UiStack};

/// Configuration for the UI debug overlay
///
/// Can be added as a `Component` to individual UI node entities.
/// This overwrites the default [`UiDebugOverlay`] resource.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct UiDebugOutline {
    /// Set to true to enable the UI debug overlay
    pub enabled: bool,
    /// Show outlines for the border boxes of UI nodes
    pub outline_border_box: bool,
    /// Show outlines for the padding boxes of UI nodes
    pub outline_padding_box: bool,
    /// Show outlines for the content boxes of UI nodes
    pub outline_content_box: bool,
    /// Show outlines for the scrollbar regions of UI nodes
    pub outline_scrollbars: bool,
    /// Width of the overlay's lines in logical pixels
    pub line_width: f32,
    /// Override Color for the overlay's lines
    pub line_color_override: Option<LinearRgba>,
    /// Show outlines for non-visible UI nodes
    pub show_hidden: bool,
    /// Show outlines for clipped sections of UI nodes
    pub show_clipped: bool,
    /// Draw outlines with sharp corners even if the UI nodes have border radii
    pub ignore_border_radius: bool,
}

impl UiDebugOutline {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

impl Default for UiDebugOutline {
    fn default() -> Self {
        Self {
            enabled: false,
            line_width: 1.,
            line_color_override: None,
            show_hidden: false,
            show_clipped: false,
            ignore_border_radius: false,
            outline_border_box: true,
            outline_padding_box: false,
            outline_content_box: false,
            outline_scrollbars: false,
        }
    }
}

impl From<UiDebugOverlay> for UiDebugOutline {
    fn from(other: UiDebugOverlay) -> Self {
        other.0.clone()
    }
}

/// Configuration for the UI debug overlay
///
/// A global `resource` that can be overridden by local component [`UiDebugOutline`] override on individual UI node entities
#[derive(Default, Resource, Reflect, Clone, Deref, DerefMut)]
#[reflect(Resource)]
pub struct UiDebugOverlay(pub UiDebugOutline);

impl From<UiDebugOutline> for UiDebugOverlay {
    fn from(other: UiDebugOutline) -> Self {
        Self(other)
    }
}

/// The debug visualization is just outlines, so it can be extracted as a single phase item for each extracted camera.
#[derive(Resource, Default)]
pub struct ExtractedUiDebugOverlay {
    /// z_index of whole visualization.
    pub z_offset: f32,
    pub default_outline: UiDebugOutline,
    pub extracted_camera_view_to_ids: EntityHashMap<(Entity, MainEntity)>,
    pub per_node_outline: MainEntityHashMap<UiDebugOutline>,
    pub changed_this_frame: MainEntityHashSet,
}

impl ExtractedUiDebugOverlay {
    pub fn get(&self, main_entity: &MainEntity) -> &UiDebugOutline {
        &self
            .per_node_outline
            .get(main_entity)
            .unwrap_or(&self.default_outline)
    }
}

pub fn extract_debug_overlay(
    mut commands: Commands,
    global_debug_options: Extract<Res<UiDebugOverlay>>,
    mut extracted_debug_layer: ResMut<ExtractedUiDebugOverlay>,
    camera_query: Extract<Query<(Entity, RenderEntity), Or<(With<Camera2d>, With<Camera3d>)>>>,
    ui_debug_outlines_query: Extract<Query<(Entity, &UiDebugOutline), Changed<UiDebugOutline>>>,
    mut removed_debug_options: Extract<RemovedComponents<UiDebugOutline>>,
    ui_stack: Extract<Res<UiStack>>,
    mut live_camera_views: Local<EntityHashSet>,
) {
    extracted_debug_layer.changed_this_frame.clear();
    extracted_debug_layer.z_offset = ui_stack.uinodes.len() as f32;
    extracted_debug_layer.default_outline = global_debug_options.0.clone();

    live_camera_views.clear();
    for (main_entity, extracted_camera_view) in camera_query.iter() {
        live_camera_views.insert(extracted_camera_view);
        extracted_debug_layer
            .extracted_camera_view_to_ids
            .entry(extracted_camera_view)
            .or_insert_with(|| (commands.spawn_empty().id(), main_entity.into()));
    }
    extracted_debug_layer.extracted_camera_view_to_ids.retain(
        |extracted_camera_view, (render_entity, _)| {
            if live_camera_views.contains(extracted_camera_view) {
                true
            } else {
                commands.entity(*render_entity).despawn();
                false
            }
        },
    );

    // iter through all nodes with UiDebugOptions
    // add to processed this frame list, so they aren't removed if tagged by removal detection
    for (entity, debug_outlines) in ui_debug_outlines_query.iter() {
        let main_entity = MainEntity::from(entity);
        extracted_debug_layer.changed_this_frame.insert(main_entity);

        extracted_debug_layer
            .per_node_outline
            .insert(main_entity, debug_outlines.clone());
    }

    for main_entity in removed_debug_options.read().map(MainEntity::from) {
        if extracted_debug_layer
            .changed_this_frame
            .contains(&main_entity)
        {
            continue;
        }
        extracted_debug_layer.per_node_outline.remove(&main_entity);
    }
}

pub fn queue_debug_overlay(
    extracted_overlays: Res<ExtractedUiDebugOverlay>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    render_views: Query<(Entity, &UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();

    for (entity, default_camera_view, ui_anti_alias) in render_views.iter() {
        let mut current_phase = camera_views
            .get(default_camera_view.0)
            .ok()
            .and_then(|view| {
                transparent_render_phases
                    .get_mut(&view.retained_view_entity)
                    .map(|transparent_phase| {
                        let pipeline = pipelines.specialize(
                            &pipeline_cache,
                            &ui_pipeline,
                            UiPipelineKey {
                                target_format: view.target_format,
                                anti_alias: matches!(ui_anti_alias, None | Some(UiAntiAlias::On)),
                            },
                        );
                        (pipeline, transparent_phase)
                    })
            });

        let Some((pipeline, transparent_phase)) = current_phase.as_mut() else {
            continue;
        };

        let Some(item_ids) = extracted_overlays.extracted_camera_view_to_ids.get(&entity) else {
            continue;
        };

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline: *pipeline,
            entity: *item_ids,
            sort_key: FloatOrd(extracted_overlays.z_offset),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}

/// Clip each debug outline quad, compute its vertices, and push them onto the vertex buffer
fn push_debug_outline(
    rect: Rect,
    border_radius: ResolvedBorderRadius,
    (line_color, line_width, clip, transform): &(LinearRgba, f32, Option<&CalculatedClip>, Affine2),
    ui_meta: &mut UiMeta,
) {
    let size = rect.size();
    let transform = transform * Affine2::from_translation(rect.center());
    let points = QUAD_VERTEX_POSITIONS.map(|pos| pos * size);
    let positions = points.map(|pos| transform.transform_point2(pos));
    let vertices = clip_polygon(
        *clip,
        &[
            (positions[0], (QUAD_UVS[0], points[0])),
            (positions[1], (QUAD_UVS[1], points[1])),
            (positions[2], (QUAD_UVS[2], points[2])),
            (positions[3], (QUAD_UVS[3], points[3])),
        ],
        |a, b, t| (a.0.lerp(b.0, t), a.1.lerp(b.1, t)),
    );
    if vertices.is_empty() {
        return;
    }

    let flags = shader_flags::UNTEXTURED | shader_flags::BORDER_ALL;
    let vertex_start = ui_meta.vertices.len() as u32;
    let color = line_color.to_f32_array();

    for &(position, (uv, point)) in &vertices {
        ui_meta.vertices.push(UiVertex {
            position: position.extend(0.).into(),
            uv: uv.into(),
            color,
            flags,
            radius: border_radius.into(),
            border: [*line_width; 4],
            size: size.into(),
            point: point.into(),
        });
    }
    for i in 1..vertices.len() as u32 - 1 {
        ui_meta.indices.push(vertex_start);
        ui_meta.indices.push(vertex_start + i);
        ui_meta.indices.push(vertex_start + i + 1);
    }
}

/// The debug overlay consists of just outlines drawn above the UI, so one phase item can hold
/// all the outlines per camera
pub fn push_debug_overlay_vertices(
    ui_meta: &mut UiMeta,
    extracted_ui_layout: &ExtractedUiLayout,
    extracted_ui_debug_overlay: &ExtractedUiDebugOverlay,
    extracted_camera: Entity,
) {
    if !extracted_ui_debug_overlay.default_outline.enabled
        && extracted_ui_debug_overlay.per_node_outline.is_empty()
    {
        return;
    }

    for (main_entity, layout) in extracted_ui_layout.layout.iter() {
        if layout.extracted_camera != extracted_camera {
            continue;
        }

        let debug_outline = extracted_ui_debug_overlay.get(main_entity);
        if !debug_outline.enabled {
            continue;
        }

        if !layout.visible && !debug_outline.show_hidden {
            continue;
        }

        let style = &(
            debug_outline
                .line_color_override
                .unwrap_or_else(|| Hsla::sequential_dispersed(main_entity.index_u32()).into()),
            debug_outline.line_width / layout.uinode.inverse_scale_factor(),
            layout.clip.as_ref().filter(|_| !debug_outline.show_clipped),
            layout.transform,
        );

        if debug_outline.outline_border_box {
            push_debug_outline(
                layout.uinode.border_box(),
                layout.uinode.border_radius,
                style,
                ui_meta,
            );
        }

        if debug_outline.outline_padding_box {
            push_debug_outline(
                layout.uinode.padding_box(),
                layout.uinode.inner_radius(),
                style,
                ui_meta,
            );
        }

        if debug_outline.outline_content_box {
            push_debug_outline(
                layout.uinode.content_box(),
                ResolvedBorderRadius::ZERO,
                style,
                ui_meta,
            );
        }

        if debug_outline.outline_scrollbars {
            if let Some((gutter, [thumb_min, thumb_max])) = layout.uinode.horizontal_scrollbar() {
                push_debug_outline(gutter, ResolvedBorderRadius::ZERO, style, ui_meta);
                push_debug_outline(
                    Rect {
                        min: Vec2::new(thumb_min, gutter.min.y),
                        max: Vec2::new(thumb_max, gutter.max.y),
                    },
                    ResolvedBorderRadius::ZERO,
                    style,
                    ui_meta,
                );
            }
            if let Some((gutter, [thumb_min, thumb_max])) = layout.uinode.vertical_scrollbar() {
                push_debug_outline(gutter, ResolvedBorderRadius::ZERO, style, ui_meta);
                push_debug_outline(
                    Rect {
                        min: Vec2::new(gutter.min.x, thumb_min),
                        max: Vec2::new(gutter.max.x, thumb_max),
                    },
                    ResolvedBorderRadius::ZERO,
                    style,
                    ui_meta,
                );
            }
        }
    }
}
