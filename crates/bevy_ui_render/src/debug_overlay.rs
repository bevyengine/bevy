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
    entity::{hash_map, Entity},
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
    ComputedUiTargetCamera, ResolvedBorderRadius, UiStack,
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
#[derive(Resource, Default)]
pub struct ExtractedUiDebugOptions {
    pub stack_offset: f32,
    pub global: UiDebugOptions,
    pub local: MainEntityHashMap<(Entity, UiDebugOptions)>,
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
    extracted_debug_options.stack_offset = ui_stack.uinodes.len() as f32;
    nodes_processed_this_frame.clear();

    extracted_debug_options.global = global_debug_options.0.clone();

    // iter through all nodes with UiDebugOptions
    // add to processed this frame list, so they aren't removed if tagged by removal detection
    for (entity, local_debug_options) in ui_debug_options_query.iter() {
        let main_entity = MainEntity::from(entity);
        nodes_processed_this_frame.insert(main_entity);

        match extracted_debug_options.local.entry(main_entity) {
            bevy_platform::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().1 = local_debug_options.clone();
            }
            bevy_platform::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert((commands.spawn_empty().id(), local_debug_options.clone()));
            }
        }
    }

    for main_entity in removed_debug_options.read().map(MainEntity::from) {
        if nodes_processed_this_frame.contains(&main_entity) {
            continue;
        }
        if let Some((render_entity, _)) = extracted_debug_options.local.remove(&main_entity) {
            commands.entity(render_entity).despawn();
        }
    }
}

pub fn queue_debug_overlay(
    extracted_overlays: Res<ExtractedUiDebugOptions>,
    extracted_layout: Res<ExtractedUiLayout>,
    ui_pipeline: Res<UiPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    render_views: Query<(&UiCameraView, Option<&UiAntiAlias>), With<ExtractedView>>,
    camera_views: Query<&ExtractedView>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUi>();
    let mut current_camera_entity = Entity::PLACEHOLDER;
    let mut current_phase = None;

    for (main_entity, (render_entity, overlay)) in &extracted_overlays.local {
        let Some(geometry) = extracted_layout.layout.get(main_entity) else {
            continue;
        };

        if current_camera_entity != geometry.extracted_camera {
            current_phase = render_views.get(geometry.extracted_camera).ok().and_then(
                |(default_camera_view, ui_anti_alias)| {
                    camera_views
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
                                            anti_alias: matches!(
                                                ui_anti_alias,
                                                None | Some(UiAntiAlias::On)
                                            ),
                                        },
                                    );
                                    (pipeline, transparent_phase)
                                })
                        })
                },
            );
            current_camera_entity = geometry.extracted_camera;
        }
        let Some((pipeline, transparent_phase)) = current_phase.as_mut() else {
            continue;
        };

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline: *pipeline,
            entity: (*render_entity, *main_entity),
            sort_key: FloatOrd(extracted_overlays.stack_offset + geometry.stack_index as f32),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
    }
}

pub fn prepare_debug_overlay() {}
