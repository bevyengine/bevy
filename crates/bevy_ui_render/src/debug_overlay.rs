use crate::{
    extract_layout::ExtractedUiLayout, shader_flags, DrawUi, ImageNodeBindGroups, TransparentUi,
    UiAntiAlias, UiBatch, UiCameraMap, UiCameraView, UiMeta, UiPipeline, UiPipelineKey, UiVertex,
    QUAD_INDICES, QUAD_VERTEX_POSITIONS,
};
use bevy_asset::AssetId;
use bevy_camera::visibility::InheritedVisibility;
use bevy_color::{ColorToComponents, Hsla, LinearRgba};
use bevy_ecs::{
    entity::Entity, lifecycle::RemovedComponents, prelude::*, query::Changed, resource::Resource,
};
use bevy_image::Image;
use bevy_math::{Affine2, FloatOrd, Rect, Vec2};
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
    ComputedUiTargetCamera, ResolvedBorderRadius, UiStack,
};

/// Configuration for the UI debug overlay
///
/// Can be added as a `Component` to individual UI node entities.
/// This overwrites the default [`GlobalUiDebugOptions`] resource.
#[derive(Component, Reflect, Copy, Clone)]
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
        Self {
            enabled: other.enabled,
            outline_border_box: other.outline_border_box,
            outline_padding_box: other.outline_padding_box,
            outline_content_box: other.outline_content_box,
            outline_scrollbars: other.outline_scrollbars,
            line_width: other.line_width,
            line_color_override: other.line_color_override,
            show_hidden: other.show_hidden,
            show_clipped: other.show_clipped,
            ignore_border_radius: other.ignore_border_radius,
        }
    }
}

/// Configuration for the UI debug overlay
///
/// A global `resource` that can be overridden by local component [`UiDebugOptions`] override on individual UI node entities
#[derive(Resource, Reflect, Copy, Clone)]
#[reflect(Resource)]
pub struct GlobalUiDebugOptions {
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

impl GlobalUiDebugOptions {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }
}

impl Default for GlobalUiDebugOptions {
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

impl From<UiDebugOptions> for GlobalUiDebugOptions {
    fn from(other: UiDebugOptions) -> Self {
        Self {
            enabled: other.enabled,
            outline_border_box: other.outline_border_box,
            outline_padding_box: other.outline_padding_box,
            outline_content_box: other.outline_content_box,
            outline_scrollbars: other.outline_scrollbars,
            line_width: other.line_width,
            line_color_override: other.line_color_override,
            show_hidden: other.show_hidden,
            show_clipped: other.show_clipped,
            ignore_border_radius: other.ignore_border_radius,
        }
    }
}

pub(super) struct ExtractedUiDebugOverlay {
    extracted_camera: Entity,
    pub(super) transform: Affine2,
    pub(super) clip: Option<Rect>,
    pub(super) color: LinearRgba,
    pub(super) border: BorderRect,
    pub(super) outlines: Vec<(Rect, ResolvedBorderRadius)>,
    z_order: f32,
}

#[derive(Resource, Default)]
pub(super) struct ExtractedUiDebugOverlays {
    pub(super) overlays: MainEntityHashMap<(Entity, ExtractedUiDebugOverlay)>,
}

pub(super) fn extract_debug_overlay(
    mut commands: Commands,
    debug_options: Extract<Res<GlobalUiDebugOptions>>,
    extracted_layout: Res<ExtractedUiLayout>,
    mut extracted_overlays: ResMut<ExtractedUiDebugOverlays>,
    changed_debug_options_query: Extract<Query<Entity, Changed<UiDebugOptions>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            Option<&UiDebugOptions>,
        )>,
    >,
    ui_stack: Extract<Res<UiStack>>,
    camera_map: Extract<UiCameraMap>,
    mut removed_debug_options: Extract<RemovedComponents<UiDebugOptions>>,
    mut nodes_to_extract: Local<MainEntityHashSet>,
) {
    nodes_to_extract.clear();
    nodes_to_extract.extend(extracted_layout.changed.iter().copied());
    nodes_to_extract.extend(changed_debug_options_query.iter().map(MainEntity::from));
    nodes_to_extract.extend(removed_debug_options.read().map(MainEntity::from));
    if debug_options.is_changed() || ui_stack.is_changed() {
        nodes_to_extract.extend(
            uinode_query
                .iter()
                .map(|(entity, ..)| MainEntity::from(entity)),
        );
    }

    let mut camera_mapper = camera_map.get_mapper();

    for main_entity in nodes_to_extract.drain() {
        let Ok((
            entity,
            uinode,
            stack_index,
            transform,
            visibility,
            maybe_clip,
            computed_target,
            local_debug_options,
        )) = uinode_query.get(main_entity.entity())
        else {
            if let Some((render_entity, _)) = extracted_overlays.overlays.remove(&main_entity) {
                commands.entity(render_entity).despawn();
            }
            continue;
        };

        let debug_options = local_debug_options
            .copied()
            .unwrap_or((*debug_options.as_ref()).into());
        if !debug_options.enabled || (!debug_options.show_hidden && !visibility.get()) {
            if let Some((render_entity, _)) = extracted_overlays.overlays.remove(&main_entity) {
                commands.entity(render_entity).despawn();
            }
            continue;
        }

        let Some(extracted_camera) = camera_mapper.map(computed_target) else {
            if let Some((render_entity, _)) = extracted_overlays.overlays.remove(&main_entity) {
                commands.entity(render_entity).despawn();
            }
            continue;
        };

        let mut outlines = Vec::with_capacity(7);
        let border_box = Rect::from_center_size(Vec2::ZERO, uinode.size());
        if debug_options.outline_border_box && !border_box.is_empty() {
            outlines.push((border_box, uinode.border_radius()));
        }
        if debug_options.outline_padding_box {
            let mut padding_box = border_box;
            padding_box.min += uinode.border().min_inset;
            padding_box.max -= uinode.border().max_inset;
            if !padding_box.is_empty() {
                outlines.push((padding_box, uinode.inner_radius()));
            }
        }
        if debug_options.outline_content_box {
            let mut content_box = border_box;
            let content_inset = uinode.content_inset();
            content_box.min += content_inset.min_inset;
            content_box.max -= content_inset.max_inset;
            if !content_box.is_empty() {
                outlines.push((content_box, ResolvedBorderRadius::ZERO));
            }
        }
        if debug_options.outline_scrollbars {
            if let Some((gutter, [thumb_min, thumb_max])) = uinode.horizontal_scrollbar() {
                if !gutter.is_empty() {
                    outlines.push((gutter, ResolvedBorderRadius::ZERO));
                }
                let thumb = Rect {
                    min: Vec2::new(thumb_min, gutter.min.y),
                    max: Vec2::new(thumb_max, gutter.max.y),
                };
                if !thumb.is_empty() {
                    outlines.push((thumb, ResolvedBorderRadius::ZERO));
                }
            }
            if let Some((gutter, [thumb_min, thumb_max])) = uinode.vertical_scrollbar() {
                if !gutter.is_empty() {
                    outlines.push((gutter, ResolvedBorderRadius::ZERO));
                }
                let thumb = Rect {
                    min: Vec2::new(gutter.min.x, thumb_min),
                    max: Vec2::new(gutter.max.x, thumb_max),
                };
                if !thumb.is_empty() {
                    outlines.push((thumb, ResolvedBorderRadius::ZERO));
                }
            }
        }

        if outlines.is_empty() {
            if let Some((render_entity, _)) = extracted_overlays.overlays.remove(&main_entity) {
                commands.entity(render_entity).despawn();
            }
            continue;
        }

        let overlay = ExtractedUiDebugOverlay {
            extracted_camera,
            transform: transform.affine(),
            clip: maybe_clip
                .filter(|_| !debug_options.show_clipped)
                .map(|clip| clip.clip),
            color: debug_options
                .line_color_override
                .unwrap_or_else(|| Hsla::sequential_dispersed(entity.index_u32()).into()),
            border: BorderRect::all(debug_options.line_width / uinode.inverse_scale_factor()),
            outlines,
            z_order: (ui_stack.uinodes.len() as u32 + stack_index.0) as f32,
        };

        match extracted_overlays.overlays.entry(main_entity) {
            bevy_platform::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().1 = overlay;
            }
            bevy_platform::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert((commands.spawn_empty().id(), overlay));
            }
        }
    }
}

pub(super) fn queue_debug_overlay(
    extracted_overlays: Res<ExtractedUiDebugOverlays>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    render_views: Query<(&UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();

    for (main_entity, (render_entity, overlay)) in &extracted_overlays.overlays {
        let Ok((default_camera_view, ui_anti_alias)) = render_views.get(overlay.extracted_camera)
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
            &ui_pipeline,
            UiPipelineKey {
                target_format: view.target_format,
                anti_alias: matches!(ui_anti_alias, None | Some(UiAntiAlias::On)),
            },
        );

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline,
            entity: (*render_entity, *main_entity),
            sort_key: FloatOrd(overlay.z_order),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}
