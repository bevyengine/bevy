use std::arch::global_asm;

use crate::{
    extract_layout::ExtractedUiLayout, shader_flags, DrawUi, ImageNodeBindGroups, TransparentUi,
    UiAntiAlias, UiBatch, UiCameraMap, UiCameraView, UiMeta, UiPipeline, UiPipelineKey, UiVertex,
    QUAD_INDICES, QUAD_VERTEX_POSITIONS,
};
use bevy_asset::AssetId;
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::{ColorToComponents, Hsla, LinearRgba};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::{hash_map, Entity, EntityHashMap},
    lifecycle::RemovedComponents,
    prelude::*,
    query::{Changed, EcsAccessLevel},
    resource::Resource,
};
use bevy_image::Image;
use bevy_math::{Affine2, FloatOrd, Rect, Vec2};
use bevy_platform::collections::hash_table::Entry;
use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{DrawFunctions, PhaseItem, PhaseItemExtraIndex, ViewSortedRenderPhases},
    render_resource::{BindGroupEntries, PipelineCache, SpecializedRenderPipelines},
    renderer::RenderDevice,
    sync_world::{MainEntity, MainEntityHashMap, MainEntityHashSet},
    texture::GpuImage,
    view::ExtractedView,
    Extract,
};
use bevy_sprite::BorderRect;
use bevy_ui::{
    ui_transform::UiGlobalTransform, CalculatedClip, ComputedNode, ComputedStackIndex,
    ComputedUiTargetCamera, ResolvedBorderRadius, UiStack, UiTargetCamera,
};

/// Configuration for the UI debug overlay
///
/// Can be added as a `Component` to individual UI node entities.
/// This overwrites the default [`GlobalUiDebugOptions`] resource.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct UiDebugOptions {
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

impl UiDebugOptions {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

impl Default for UiDebugOptions {
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

impl From<GlobalUiDebugOptions> for UiDebugOptions {
    fn from(other: GlobalUiDebugOptions) -> Self {
        other.0.clone()
    }
}

/// Configuration for the UI debug overlay
///
/// A global `resource` that can be overridden by local component [`UiDebugOptions`] override on individual UI node entities
#[derive(Default, Resource, Reflect, Clone, Deref, DerefMut)]
#[reflect(Resource)]
pub struct GlobalUiDebugOptions(pub UiDebugOptions);

impl From<UiDebugOptions> for GlobalUiDebugOptions {
    fn from(other: UiDebugOptions) -> Self {
        Self(other)
    }
}

/// The debug visualization is just outlines, so it can be extracted as a single phase item for each extracted camera.
#[derive(Resource, Default)]
pub struct ExtractedUiDebugOptions {
    /// z_index of whole visualization.
    pub z_offset: f32,
    pub global_debug_options: UiDebugOptions,
    /// Map from extracted view -> (render entity for debug phase item, Node MainEntity -> UiDebugOptions)
    pub local_debug_options: MainEntityHashMap<UiDebugOptions>,
}

pub fn extract_debug_overlay(
    mut commands: Commands,
    global_debug_options: Extract<Res<GlobalUiDebugOptions>>,
    mut extracted_debug_options: ResMut<ExtractedUiDebugOptions>,
    ui_debug_options_query: Extract<Query<(Entity, &UiDebugOptions), Changed<UiDebugOptions>>>,
    mut removed_debug_options: Extract<RemovedComponents<UiDebugOptions>>,
    mut nodes_processed_this_frame: Local<MainEntityHashSet>,
    ui_stack: Res<UiStack>,
) {
    extracted_debug_options.z_offset = ui_stack.uinodes.len() as f32;
    nodes_processed_this_frame.clear();

    extracted_debug_options.global_debug_options = global_debug_options.0.clone();

    // iter through all nodes with UiDebugOptions
    // add to processed this frame list, so they aren't removed if tagged by removal detection
    for (entity, local_debug_options) in ui_debug_options_query.iter() {
        let main_entity = MainEntity::from(entity);
        nodes_processed_this_frame.insert(main_entity);

        extracted_debug_options
            .local_debug_options
            .insert(main_entity, local_debug_options.clone());
    }

    for main_entity in removed_debug_options.read().map(MainEntity::from) {
        if nodes_processed_this_frame.contains(&main_entity) {
            continue;
        }
        extracted_debug_options
            .local_debug_options
            .remove(&main_entity);
    }
}

pub fn queue_debug_overlay(
    extracted_overlays: Res<ExtractedUiDebugOptions>,
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

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline: *pipeline,
            entity: (entity, entity.into()),
            sort_key: FloatOrd(extracted_overlays.z_offset),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}

/// Debug layout can just
fn generate_debug_layout_quads(
    ui_meta: &mut UiMeta,
    layout: &ExtractedUiNodeLayout,
    style: &ExtractedUiNodeStyle,
    image: AssetId<Image>,
    gpu_image: &GpuImage,
) {
    let uinode = &layout.uinode;
    let mut push_quad = |color: LinearRgba,
                         mut rect: Rect,
                         atlas_scaling: Option<Vec2>,
                         transform: Affine2,
                         clip: Option<Rect>,
                         border_radius: ResolvedBorderRadius,
                         border: BorderRect,
                         node_type: NodeType,
                         textured: bool| {
        let mut flags = if textured {
            shader_flags::TEXTURED
        } else {
            shader_flags::UNTEXTURED
        };
        let rect_size = rect.size();
        let positions =
            QUAD_VERTEX_POSITIONS.map(|pos| transform.transform_point2(pos * rect_size).extend(0.));
        let mut positions_diff = if let Some(clip) = clip {
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
        let transformed_rect_size = transform.transform_vector2(rect_size).abs();
        if transform.x_axis[1] == 0.0
            && (positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y)
        {
            return;
        }
        let uvs = if textured {
            let atlas_extent = atlas_scaling
                .map(|scaling| gpu_image.size_2d().as_vec2() * scaling)
                .unwrap_or(rect.max);
            if style.flip_x {
                mem::swap(&mut rect.max.x, &mut rect.min.x);
                positions_diff[0].x *= -1.;
                positions_diff[1].x *= -1.;
                positions_diff[2].x *= -1.;
                positions_diff[3].x *= -1.;
            }
            if style.flip_y {
                mem::swap(&mut rect.max.y, &mut rect.min.y);
                positions_diff[0].y *= -1.;
                positions_diff[1].y *= -1.;
                positions_diff[2].y *= -1.;
                positions_diff[3].y *= -1.;
            }
            [
                Vec2::new(
                    rect.min.x + positions_diff[0].x,
                    rect.min.y + positions_diff[0].y,
                ),
                Vec2::new(
                    rect.max.x + positions_diff[1].x,
                    rect.min.y + positions_diff[1].y,
                ),
                Vec2::new(
                    rect.max.x + positions_diff[2].x,
                    rect.max.y + positions_diff[2].y,
                ),
                Vec2::new(
                    rect.min.x + positions_diff[3].x,
                    rect.max.y + positions_diff[3].y,
                ),
            ]
            .map(|pos| pos / atlas_extent)
        } else {
            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
        };
        match node_type {
            NodeType::Border(border_flags) => flags |= border_flags,
            NodeType::Inverted => flags |= INVERT,
            NodeType::Rect => {}
        }

        let vertex_start = ui_meta.vertices.len() as u32;
        let color = color.to_f32_array();
        for i in 0..4 {
            ui_meta.vertices.push(UiVertex {
                position: positions_clipped[i].into(),
                uv: uvs[i].into(),
                color,
                flags: flags | shader_flags::CORNERS[i],
                radius: border_radius.into(),
                border: [
                    border.min_inset.x,
                    border.min_inset.y,
                    border.max_inset.x,
                    border.max_inset.y,
                ],
                size: rect_size.into(),
                point: (QUAD_VERTEX_POSITIONS[i] * rect_size + positions_diff[i]).into(),
            });
        }
        for &index in &QUAD_INDICES {
            ui_meta.indices.push(vertex_start + index as u32);
        }
    };

    if !style.background_color.is_fully_transparent() {
        push_quad(
            style.background_color,
            Rect {
                min: Vec2::ZERO,
                max: uinode.size(),
            },
            None,
            layout.transform,
            layout.clip,
            uinode.border_radius(),
            uinode.border(),
            NodeType::Rect,
            false,
        );
    }
    if !style.outer_color.is_fully_transparent() {
        push_quad(
            style.outer_color,
            Rect {
                min: Vec2::ZERO,
                max: uinode.size(),
            },
            None,
            layout.transform,
            layout.clip,
            uinode.border_radius(),
            BorderRect::ZERO,
            NodeType::Inverted,
            false,
        );
    }

    if uinode.border() != BorderRect::ZERO {
        const BORDER_FLAGS: [u32; 4] = [
            shader_flags::BORDER_TOP,
            shader_flags::BORDER_RIGHT,
            shader_flags::BORDER_BOTTOM,
            shader_flags::BORDER_LEFT,
        ];
        let mut completed_flags = 0;

        for (i, &color) in style.border_color.iter().enumerate() {
            if color.is_fully_transparent() || completed_flags & BORDER_FLAGS[i] != 0 {
                continue;
            }

            let mut border_flags = BORDER_FLAGS[i];
            for (j, &other_color) in style.border_color.iter().enumerate().skip(i + 1) {
                if color == other_color {
                    border_flags |= BORDER_FLAGS[j];
                }
            }
            completed_flags |= border_flags;

            push_quad(
                color,
                Rect {
                    min: Vec2::ZERO,
                    max: uinode.size(),
                },
                None,
                layout.transform,
                layout.clip,
                uinode.border_radius(),
                uinode.border(),
                NodeType::Border(border_flags),
                false,
            );
        }
    }

    if !style.outline_color.is_fully_transparent() && uinode.outline_width() > 0. {
        push_quad(
            style.outline_color,
            Rect {
                min: Vec2::ZERO,
                max: uinode.outlined_node_size(),
            },
            None,
            layout.transform,
            layout.clip,
            uinode.outline_radius(),
            BorderRect::all(uinode.outline_width()),
            NodeType::Border(shader_flags::BORDER_ALL),
            false,
        );
    }

    if style.image == Some(image) {
        let visual_box = match style.visual_box {
            VisualBox::ContentBox => uinode.content_box(),
            VisualBox::PaddingBox => uinode.padding_box(),
            VisualBox::BorderBox => uinode.border_box(),
        };
        if !visual_box.size().cmple(Vec2::ZERO).any() {
            let size = if style.auto_sized && !style.image_size.cmple(Vec2::ZERO).any() {
                style.image_size * (visual_box.size() / style.image_size).min_element()
            } else {
                visual_box.size()
            };
            let mut rect = style.image_rect.unwrap_or(Rect {
                min: Vec2::ZERO,
                max: size,
            });
            let atlas_scaling = style.image_rect.map(|_| {
                let atlas_scaling = size / rect.size();
                rect.min *= atlas_scaling;
                rect.max *= atlas_scaling;
                atlas_scaling
            });
            push_quad(
                style.image_color,
                rect,
                atlas_scaling,
                layout.transform * Affine2::from_translation(visual_box.center()),
                layout.clip,
                uinode.border_radius(),
                if style.use_node_border {
                    uinode.border()
                } else {
                    BorderRect::ZERO
                },
                NodeType::Rect,
                true,
            );
        }
    }
}
