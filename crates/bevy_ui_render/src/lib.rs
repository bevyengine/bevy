#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Provides rendering functionality for `bevy_ui`.

pub mod box_shadow;
mod gradient;
mod image;
mod pipeline;
pub mod render_pass;
mod text;
pub mod ui_material;
mod ui_material_pipeline;
pub mod ui_texture_slice_pipeline;

#[cfg(feature = "bevy_ui_debug")]
mod debug_overlay;

use bevy_a11y::AccessibilitySystems;
use bevy_camera::visibility::InheritedVisibility;
use bevy_camera::{Camera, Camera2d, Camera3d, RenderTarget};
use bevy_ecs::entity::EntityIndexMap;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::camera::{extract_cameras, CameraMainPassTextureFormats};
use bevy_render::sync_world::{MainEntityHashMap, MainEntityHashSet};
use bevy_shader::load_shader_library;
use bevy_sprite_render::SpriteAssetEvents;
use bevy_ui::widget::{ImageNode, ImageNodeSize, NodeImageMode, Text, TextShadow, ViewportNode};
use bevy_ui::{
    BackgroundColor, BackgroundGradient, BorderColor, BorderGradient, BoxShadow, CalculatedClip,
    ComputedNode, ComputedStackIndex, ComputedUiTargetCamera, Display, Node, OuterColor, Outline,
    ResolvedBorderRadius, UiGlobalTransform, UiSystems, VisualBox,
};

use bevy_app::prelude::*;
use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets};
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_core_pipeline::schedule::{Core2d, Core2dSystems, Core3d, Core3dSystems};
use bevy_core_pipeline::upscaling::upscaling;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::SystemParam;
use bevy_image::{prelude::*, TRANSPARENT_IMAGE_HANDLE};
use bevy_math::{proj, Affine2, FloatOrd, Rect, UVec4, Vec2};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{
        sort_phase_system, AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity},
    texture::GpuImage,
    view::{ExtractedView, RetainedViewEntity, ViewUniforms},
    Extract, ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_sprite::BorderRect;
#[cfg(feature = "bevy_ui_debug")]
pub use debug_overlay::{GlobalUiDebugOptions, UiDebugOptions};

use gradient::GradientPlugin;

use bevy_platform::collections::{HashMap, HashSet};
use bevy_text::{
    ComputedTextBlock, EditableText, PositionedGlyph, Strikethrough, StrikethroughColor,
    TextBackgroundColor, TextColor, TextCursorStyle, TextLayoutInfo, TextSpan, Underline,
    UnderlineColor,
};
use bevy_transform::components::GlobalTransform;
use box_shadow::BoxShadowPlugin;
use bytemuck::{Pod, Zeroable};
use core::ops::Range;
use std::mem;

pub use pipeline::*;
pub use render_pass::*;
pub use ui_material_pipeline::*;
use ui_texture_slice_pipeline::UiTextureSlicerPlugin;

use crate::shader_flags::INVERT;
use crate::text::{extract_preedit_underlines, extract_text_cursor};

pub mod prelude {
    #[cfg(feature = "bevy_ui_debug")]
    pub use crate::debug_overlay::{GlobalUiDebugOptions, UiDebugOptions};

    pub use crate::{
        ui_material::*, ui_material_pipeline::UiMaterialPlugin, BoxShadowSamples, UiAntiAlias,
    };
}

/// Local Z offsets of "extracted nodes" for a given entity. These exist to allow rendering multiple "extracted nodes"
/// for a given source entity (ex: render both a background color _and_ a custom material for a given node).
///
/// When possible these offsets should be defined in _this_ module to ensure z-index coordination across contexts.
/// When this is _not_ possible, pick a suitably unique index unlikely to clash with other things (ex: `0.1826823` not `0.1`).
///
/// Offsets should be unique for a given node entity to avoid z fighting.
/// These should pretty much _always_ be larger than -0.5 and smaller than 0.5 to avoid clipping into nodes
/// above / below the current node in the stack.
///
/// A z-index of 0.0 is the baseline, which is used as the primary "background color" of the node.
///
/// Note that nodes "stack" on each other, so a negative offset on the node above could clip _into_
/// a positive offset on a node below.
pub mod stack_z_offsets {
    pub const BOX_SHADOW: f32 = -0.1;
    pub const BACKGROUND_COLOR: f32 = 0.0;
    pub const BORDER: f32 = 0.01;
    pub const GRADIENT: f32 = 0.02;
    pub const BORDER_GRADIENT: f32 = 0.03;
    pub const IMAGE: f32 = 0.04;
    pub const MATERIAL: f32 = 0.05;
    pub const TEXT_SELECTION: f32 = 0.055;
    pub const TEXT: f32 = 0.06;
    pub const TEXT_STRIKETHROUGH: f32 = 0.07;
    pub const TEXT_CURSOR: f32 = 0.08;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystems {
    ExtractChanges,
    ExtractCameraViews,
    ExtractBoxShadows,
    ExtractBackgrounds,
    ExtractImages,
    ExtractTextureSlice,
    ExtractBorders,
    ExtractViewportNodes,
    ExtractTextBackgrounds,
    ExtractTextShadows,
    ExtractText,
    ExtractCursor,
    ExtractDebug,
    ExtractGradient,
}

/// Marker for controlling whether UI is rendered with or without anti-aliasing
/// in a camera. By default, UI is always anti-aliased.
///
/// **Note:** This does not affect text anti-aliasing. For that, use the `font_smoothing` property of the [`TextFont`](bevy_text::TextFont) component.
///
/// ```
/// use bevy_camera::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_ui::prelude::*;
/// use bevy_ui_render::prelude::*;
///
/// fn spawn_camera(mut commands: Commands) {
///     commands.spawn((
///         Camera2d,
///         // This will cause all UI in this camera to be rendered without
///         // anti-aliasing
///         UiAntiAlias::Off,
///     ));
/// }
/// ```
#[derive(Component, Clone, Copy, Default, Debug, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, PartialEq, Clone)]
pub enum UiAntiAlias {
    /// UI will render with anti-aliasing
    #[default]
    On,
    /// UI will render without anti-aliasing
    Off,
}

/// Number of shadow samples.
/// A larger value will result in higher quality shadows.
/// Default is 4, values higher than ~10 offer diminishing returns.
///
/// ```
/// use bevy_camera::prelude::*;
/// use bevy_ecs::prelude::*;
/// use bevy_ui::prelude::*;
/// use bevy_ui_render::prelude::*;
///
/// fn spawn_camera(mut commands: Commands) {
///     commands.spawn((
///         Camera2d,
///         BoxShadowSamples(6),
///     ));
/// }
/// ```
#[derive(Component, Clone, Copy, Debug, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, PartialEq, Clone)]
pub struct BoxShadowSamples(pub u32);

impl Default for BoxShadowSamples {
    fn default() -> Self {
        Self(4)
    }
}

#[derive(Default)]
pub struct UiRenderPlugin;

impl Plugin for UiRenderPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "ui.wgsl");

        #[cfg(feature = "bevy_ui_debug")]
        app.init_resource::<GlobalUiDebugOptions>();

        app.add_systems(
            PostUpdate,
            (
                image::mark_images_as_changed_if_their_assets_changed,
                image::update_texture_atlas_layout_components,
            )
                .chain()
                .after(UiSystems::Content)
                .after(AssetEventSystems)
                .after(AccessibilitySystems::Update),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<UiPipeline>>()
            .init_gpu_resource::<ImageNodeBindGroups>()
            .init_gpu_resource::<UiMeta>()
            .init_resource::<ExtractedUiNodes>()
            .allow_ambiguous_resource::<ExtractedUiNodes>()
            .init_resource::<DrawFunctions<TransparentUi>>()
            .init_resource::<ViewSortedRenderPhases<TransparentUi>>()
            .allow_ambiguous_resource::<ViewSortedRenderPhases<TransparentUi>>()
            .add_render_command::<TransparentUi, DrawUi>()
            .configure_sets(
                ExtractSchedule,
                (
                    RenderUiSystems::ExtractChanges,
                    RenderUiSystems::ExtractCameraViews,
                    RenderUiSystems::ExtractBoxShadows,
                    RenderUiSystems::ExtractBackgrounds,
                    RenderUiSystems::ExtractViewportNodes,
                    RenderUiSystems::ExtractImages,
                    RenderUiSystems::ExtractTextureSlice,
                    RenderUiSystems::ExtractBorders,
                    RenderUiSystems::ExtractTextBackgrounds,
                    RenderUiSystems::ExtractTextShadows,
                    RenderUiSystems::ExtractText,
                    RenderUiSystems::ExtractCursor,
                    RenderUiSystems::ExtractDebug,
                )
                    .chain(),
            )
            .add_systems(RenderStartup, init_ui_pipeline)
            .add_systems(
                ExtractSchedule,
                (
                    extract_uinode_changes.in_set(RenderUiSystems::ExtractChanges),
                    extract_ui_camera_view
                        .after(extract_cameras)
                        .in_set(RenderUiSystems::ExtractCameraViews),
                    extract_uinode_background_colors.in_set(RenderUiSystems::ExtractBackgrounds),
                    extract_uinode_images.in_set(RenderUiSystems::ExtractImages),
                    extract_uinode_borders.in_set(RenderUiSystems::ExtractBorders),
                    extract_viewport_nodes.in_set(RenderUiSystems::ExtractViewportNodes),
                    extract_text_decorations.in_set(RenderUiSystems::ExtractTextBackgrounds),
                    extract_text_shadows.in_set(RenderUiSystems::ExtractTextShadows),
                    extract_text_sections.in_set(RenderUiSystems::ExtractText),
                    extract_text_cursor.in_set(RenderUiSystems::ExtractCursor),
                    extract_preedit_underlines.in_set(RenderUiSystems::ExtractCursor),
                    #[cfg(feature = "bevy_ui_debug")]
                    debug_overlay::extract_debug_overlay.in_set(RenderUiSystems::ExtractDebug),
                ),
            )
            .add_systems(
                Render,
                (
                    queue_uinodes.in_set(RenderSystems::Queue),
                    sort_phase_system::<TransparentUi>.in_set(RenderSystems::PhaseSort),
                    prepare_uinodes.in_set(RenderSystems::PrepareBindGroups),
                    clear_batches.in_set(RenderSystems::Cleanup),
                ),
            )
            .add_systems(
                Core2d,
                ui_pass.after(Core2dSystems::PostProcess).before(upscaling),
            )
            .add_systems(
                Core3d,
                ui_pass.after(Core3dSystems::PostProcess).before(upscaling),
            );

        app.add_plugins(UiTextureSlicerPlugin);
        app.add_plugins(GradientPlugin);
        app.add_plugins(BoxShadowPlugin);
    }
}

#[derive(SystemParam)]
pub struct UiCameraMap<'w, 's> {
    mapping: Query<'w, 's, RenderEntity>,
}

impl<'w, 's> UiCameraMap<'w, 's> {
    /// Creates a [`UiCameraMapper`] for performing repeated camera-to-render-entity lookups.
    ///
    /// The last successful mapping is cached to avoid redundant queries.
    pub fn get_mapper(&'w self) -> UiCameraMapper<'w, 's> {
        UiCameraMapper {
            mapping: &self.mapping,
            camera_entity: Entity::PLACEHOLDER,
            render_entity: Entity::PLACEHOLDER,
        }
    }
}

/// Helper for mapping UI target camera entities to their corresponding render entities,
/// with caching to avoid repeated lookups for the same camera.
pub struct UiCameraMapper<'w, 's> {
    mapping: &'w Query<'w, 's, RenderEntity>,
    /// Cached camera entity from the last successful `map` call.
    camera_entity: Entity,
    /// Cached camera entity from the last successful `map` call.
    render_entity: Entity,
}

impl<'w, 's> UiCameraMapper<'w, 's> {
    /// Returns the render entity corresponding to the given [`ComputedUiTargetCamera`]'s camera, or none if no corresponding entity was found.
    pub fn map(&mut self, computed_target: &ComputedUiTargetCamera) -> Option<Entity> {
        let camera_entity = computed_target.get()?;
        if self.camera_entity != camera_entity {
            let new_render_camera_entity = self.mapping.get(camera_entity).ok()?;
            self.render_entity = new_render_camera_entity;
            self.camera_entity = camera_entity;
        }

        Some(self.render_entity)
    }

    /// Returns the cached camera entity from the last successful `map` call.
    pub fn current_camera(&self) -> Entity {
        self.camera_entity
    }
}

pub struct ExtractedUiNode {
    pub z_order: f32,
    pub image: AssetId<Image>,
    pub clip: Option<Rect>,
    /// Render world entity of the extracted camera corresponding to this node's target camera.
    pub extracted_camera_entity: Entity,
    pub item: ExtractedUiItem,
    pub transform: Affine2,
}

/// The type of UI node.
/// This is used to determine how to render the UI node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeType {
    Rect,
    Inverted,
    Border(u32), // shader flags
}

pub enum ExtractedUiItem {
    Node {
        color: LinearRgba,
        rect: Rect,
        atlas_scaling: Option<Vec2>,
        flip_x: bool,
        flip_y: bool,
        /// Border radius of the UI node.
        /// Ordering: top left, top right, bottom right, bottom left.
        border_radius: ResolvedBorderRadius,
        /// Border thickness of the UI node.
        /// Ordering: left, top, right, bottom.
        border: BorderRect,
        node_type: NodeType,
    },
    /// A contiguous sequence of text glyphs from the same section
    Glyphs {
        /// The color, position, and UV rect of each glyph.
        glyphs: Vec<ExtractedGlyph>,
    },
}

pub struct ExtractedGlyph {
    pub color: LinearRgba,
    pub translation: Vec2,
    pub rect: Rect,
}

/// The list of UI nodes, as well as the set of nodes that changed.
///
/// This is a two-level data structure so that we can quickly remove all
/// gradients associated with a main-world entity when it changes.
#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    /// The list of UI nodes.
    ///
    /// This is a two-level data structure so that we can quickly remove all UI
    /// nodes associated with a main-world entity when it changes.
    pub uinodes: MainEntityHashMap<EntityIndexMap<ExtractedUiNode>>,
    /// UI nodes that changed this frame.
    pub changed: MainEntityHashSet,
}

/// A query filter that matches all UI nodes.
type UiNodeQueryFilter = (
    With<ComputedNode>,
    With<ComputedStackIndex>,
    With<UiGlobalTransform>,
    With<InheritedVisibility>,
    With<ComputedUiTargetCamera>,
);

// Note: Whenever you add a new component that affects UI rendering, make sure
// to add a `Changed` query filter and a reference to the `RemovedComponents`
// resource to `extract_uinode_changes` below.
//
// Note: We don't have to match on `AssetChanged` for images or texture atlas
// layouts because the
// `bevy_ui::widget::mark_images_as_changed_as_their_assets_changed` image marks
// the `ImageNode` for us automatically as changed when those assets change.

/// A render-world system that scans for any UI nodes that have changed and
/// removes the render world data associated with them.
pub fn extract_uinode_changes(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    all_uinodes_query: Extract<Query<Entity, UiNodeQueryFilter>>,
    changed_uinodes_query: Extract<
        Query<
            Entity,
            (
                UiNodeQueryFilter,
                Or<(
                    Or<(
                        Changed<ComputedNode>,
                        Changed<ComputedStackIndex>,
                        Changed<UiGlobalTransform>,
                        Changed<InheritedVisibility>,
                        Changed<CalculatedClip>,
                        Changed<ComputedUiTargetCamera>,
                        Changed<BackgroundColor>,
                        Changed<OuterColor>,
                    )>,
                    Or<(
                        Changed<ImageNode>,
                        Changed<ImageNodeSize>,
                        Changed<BorderColor>,
                        Changed<Outline>,
                        Changed<ViewportNode>,
                        Changed<ComputedTextBlock>,
                        Changed<TextColor>,
                        Changed<TextLayoutInfo>,
                    )>,
                    Or<(
                        Changed<TextCursorStyle>,
                        Changed<TextShadow>,
                        Changed<BackgroundGradient>,
                        Changed<BorderGradient>,
                        Changed<BoxShadow>,
                        Changed<EditableText>,
                        Changed<Underline>,
                        Changed<Strikethrough>,
                    )>,
                    Or<(Changed<StrikethroughColor>, Changed<UnderlineColor>)>,
                )>,
            ),
        >,
    >,
    #[cfg(feature = "bevy_ui_debug")] changed_debug_options_query: Extract<
        Query<Entity, (UiNodeQueryFilter, Changed<UiDebugOptions>)>,
    >,
    text_span_query: Extract<
        Query<
            Entity,
            (
                With<TextSpan>,
                Or<(
                    Changed<TextColor>,
                    Changed<TextBackgroundColor>,
                    Changed<Underline>,
                    Changed<Strikethrough>,
                    Changed<StrikethroughColor>,
                    Changed<UnderlineColor>,
                )>,
            ),
        >,
    >,
    text_span_parent_query: Extract<Query<&ChildOf, With<TextSpan>>>,
    text_query: Extract<Query<Entity, With<Text>>>,
    (
        mut removed_computed_node_query,
        mut removed_computed_stack_index_query,
        mut removed_ui_global_transform_query,
        mut removed_inherited_visibility_query,
        mut removed_calculated_clip_query,
        mut removed_computed_ui_target_camera_query,
        mut removed_background_color_query,
        mut removed_outer_color_query,
    ): (
        Extract<RemovedComponents<ComputedNode>>,
        Extract<RemovedComponents<ComputedStackIndex>>,
        Extract<RemovedComponents<UiGlobalTransform>>,
        Extract<RemovedComponents<InheritedVisibility>>,
        Extract<RemovedComponents<CalculatedClip>>,
        Extract<RemovedComponents<ComputedUiTargetCamera>>,
        Extract<RemovedComponents<BackgroundColor>>,
        Extract<RemovedComponents<OuterColor>>,
    ),
    (
        mut removed_image_node_query,
        mut removed_image_node_size_query,
        mut removed_border_color_query,
        mut removed_outline_query,
        mut removed_viewport_node_query,
        mut removed_computed_text_block_query,
        mut removed_text_color_query,
        mut removed_text_layout_info_query,
    ): (
        Extract<RemovedComponents<ImageNode>>,
        Extract<RemovedComponents<ImageNodeSize>>,
        Extract<RemovedComponents<BorderColor>>,
        Extract<RemovedComponents<Outline>>,
        Extract<RemovedComponents<ViewportNode>>,
        Extract<RemovedComponents<ComputedTextBlock>>,
        Extract<RemovedComponents<TextColor>>,
        Extract<RemovedComponents<TextLayoutInfo>>,
    ),
    (
        mut removed_text_cursor_style_query,
        mut removed_text_shadow_query,
        mut removed_background_gradient_query,
        mut removed_border_gradient_query,
        mut removed_box_shadow_query,
        mut removed_editable_text_query,
        mut removed_underline_query,
        mut removed_strikethrough_query,
    ): (
        Extract<RemovedComponents<TextCursorStyle>>,
        Extract<RemovedComponents<TextShadow>>,
        Extract<RemovedComponents<BackgroundGradient>>,
        Extract<RemovedComponents<BorderGradient>>,
        Extract<RemovedComponents<BoxShadow>>,
        Extract<RemovedComponents<EditableText>>,
        Extract<RemovedComponents<Underline>>,
        Extract<RemovedComponents<Strikethrough>>,
    ),
    (mut removed_strikethrough_color_query, mut removed_underline_color_query): (
        Extract<RemovedComponents<StrikethroughColor>>,
        Extract<RemovedComponents<UnderlineColor>>,
    ),
    #[cfg(feature = "bevy_ui_debug")] mut removed_debug_options_query: Extract<
        RemovedComponents<UiDebugOptions>,
    >,
    #[cfg(feature = "bevy_ui_debug")] global_ui_debug_options: Extract<Res<GlobalUiDebugOptions>>,
    mut extra_nodes_to_invalidate: Local<MainEntityHashSet>,
) {
    extracted_uinodes.changed.clear();

    // If the debug options changed, we wipe everything.
    // That's a bit coarse-grained, but having the debug options change is rare
    // and should only happen in, well, debugging.
    #[cfg(feature = "bevy_ui_debug")]
    let must_wipe_all_nodes = global_ui_debug_options.is_changed();
    #[cfg(not(feature = "bevy_ui_debug"))]
    let must_wipe_all_nodes = false;

    if must_wipe_all_nodes {
        for main_entity in &all_uinodes_query {
            process_changed_entity(
                main_entity.into(),
                &mut commands,
                &text_span_parent_query,
                &text_query,
                &mut extracted_uinodes,
                Some(&mut extra_nodes_to_invalidate),
            );
        }
    } else {
        // Go through all nodes that have changed and invalidate any render world
        // data associated with them.
        for main_entity in changed_uinodes_query
            .iter()
            .chain(text_span_query.iter())
            .chain(removed_computed_node_query.read())
            .chain(removed_computed_stack_index_query.read())
            .chain(removed_ui_global_transform_query.read())
            .chain(removed_inherited_visibility_query.read())
            .chain(removed_calculated_clip_query.read())
            .chain(removed_computed_ui_target_camera_query.read())
            .chain(removed_background_color_query.read())
            .chain(removed_outer_color_query.read())
            .chain(removed_image_node_query.read())
            .chain(removed_image_node_size_query.read())
            .chain(removed_border_color_query.read())
            .chain(removed_outline_query.read())
            .chain(removed_viewport_node_query.read())
            .chain(removed_computed_text_block_query.read())
            .chain(removed_text_color_query.read())
            .chain(removed_text_layout_info_query.read())
            .chain(removed_text_cursor_style_query.read())
            .chain(removed_text_shadow_query.read())
            .chain(removed_background_gradient_query.read())
            .chain(removed_border_gradient_query.read())
            .chain(removed_box_shadow_query.read())
            .chain(removed_editable_text_query.read())
            .chain(removed_underline_query.read())
            .chain(removed_strikethrough_query.read())
            .chain(removed_strikethrough_color_query.read())
            .chain(removed_underline_color_query.read())
        {
            process_changed_entity(
                main_entity.into(),
                &mut commands,
                &text_span_parent_query,
                &text_query,
                &mut extracted_uinodes,
                Some(&mut extra_nodes_to_invalidate),
            );
        }

        // Process nodes that have changed debug options too, if that feature is
        // enabled.
        #[cfg(feature = "bevy_ui_debug")]
        for main_entity in changed_debug_options_query
            .iter()
            .chain(removed_debug_options_query.read())
        {
            process_changed_entity(
                main_entity.into(),
                &mut commands,
                &text_span_parent_query,
                &text_query,
                &mut extracted_uinodes,
                Some(&mut extra_nodes_to_invalidate),
            );
        }
    }

    for main_entity in extra_nodes_to_invalidate.drain() {
        process_changed_entity(
            main_entity,
            &mut commands,
            &text_span_parent_query,
            &text_query,
            &mut extracted_uinodes,
            None,
        );
    }

    fn process_changed_entity(
        mut main_entity: MainEntity,
        commands: &mut Commands,
        text_span_parent_query: &Query<&ChildOf, With<TextSpan>>,
        text_query: &Query<Entity, With<Text>>,
        extracted_uinodes: &mut ExtractedUiNodes,
        maybe_extra_nodes_to_invalidate: Option<&mut MainEntityHashSet>,
    ) {
        // Mark the node as changed so that the other `extract_` systems will
        // know to process it.
        extracted_uinodes.changed.insert(main_entity);

        if let Some(mut render_entities) = extracted_uinodes.uinodes.remove(&main_entity) {
            for (render_entity, _) in render_entities.drain(..) {
                commands.entity(render_entity).despawn();
            }
        }

        // If this node is a `TextSpan`, then we need to invalidate the ancestor
        // `Text` node too. This is because `extract_text_decorations` only
        // looks at the `text_background_colors_query` for the text spans if the
        // `uinode_query` that it's iterating over matched the ancestor `Text`
        // node.
        if let Some(extra_nodes_to_invalidate) = maybe_extra_nodes_to_invalidate
            && let Ok(parent) = text_span_parent_query.get(main_entity.entity())
        {
            main_entity = parent.parent().into();
            loop {
                if text_query.contains(main_entity.entity()) {
                    extra_nodes_to_invalidate.insert(main_entity);
                    break;
                }
                match text_span_parent_query.get(main_entity.entity()) {
                    Ok(parent) => main_entity = parent.parent().into(),
                    Err(_) => break,
                }
            }
        }
    }
}

pub fn extract_uinode_background_colors(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &BackgroundColor,
            Option<&OuterColor>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        stack_index,
        transform,
        inherited_visibility,
        clip,
        camera,
        background_color,
        maybe_outer_color,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip invisible backgrounds
        if !inherited_visibility.get()
            || (background_color.is_fully_transparent()
                && maybe_outer_color.is_none_or(|outer| outer.is_fully_transparent()))
            || uinode.is_empty()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        if !background_color.is_fully_transparent() {
            extracted_uinodes
                .uinodes
                .entry(entity.into())
                .or_default()
                .insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        z_order: stack_index.0 as f32 + stack_z_offsets::BACKGROUND_COLOR,
                        clip: clip.map(|clip| clip.clip),
                        image: AssetId::default(),
                        extracted_camera_entity,
                        transform: transform.into(),
                        item: ExtractedUiItem::Node {
                            color: background_color.0.into(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border: uinode.border(),
                            border_radius: uinode.border_radius(),
                            node_type: NodeType::Rect,
                        },
                    },
                );
        }

        if let Some(outer_color) = maybe_outer_color
            && !outer_color.0.is_fully_transparent()
        {
            extracted_uinodes
                .uinodes
                .entry(entity.into())
                .or_default()
                .insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        z_order: stack_index.0 as f32 + stack_z_offsets::BACKGROUND_COLOR,
                        clip: clip.map(|clip| clip.clip),
                        image: AssetId::default(),
                        extracted_camera_entity,
                        transform: transform.into(),
                        item: ExtractedUiItem::Node {
                            color: outer_color.0.into(),
                            rect: Rect {
                                min: Vec2::ZERO,
                                max: uinode.size,
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border: BorderRect::ZERO,
                            border_radius: uinode.border_radius(),
                            node_type: NodeType::Inverted,
                        },
                    },
                );
        }
    }
}

pub fn extract_uinode_images(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ImageNode,
            &ImageNodeSize,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        stack_index,
        transform,
        inherited_visibility,
        clip,
        camera,
        image,
        image_size,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        let visual_box = match image.visual_box {
            VisualBox::ContentBox => uinode.content_box(),
            VisualBox::PaddingBox => uinode.padding_box(),
            VisualBox::BorderBox => uinode.border_box(),
        };
        // Skip invisible images
        if !inherited_visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
            || image.image_mode.uses_slices()
            || visual_box.size().cmple(Vec2::ZERO).any()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let size = if matches!(image.image_mode, NodeImageMode::Auto) {
            let source = image_size.size().as_vec2();
            if source.cmple(Vec2::ZERO).any() {
                visual_box.size()
            } else {
                source * (visual_box.size() / source).min_element()
            }
        } else {
            visual_box.size()
        };

        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&texture_atlases))
            .map(|r| r.as_rect());

        let mut rect = match (atlas_rect, image.rect) {
            (None, None) => Rect {
                min: Vec2::ZERO,
                max: size,
            },
            (None, Some(image_rect)) => image_rect,
            (Some(atlas_rect), None) => atlas_rect,
            (Some(atlas_rect), Some(mut image_rect)) => {
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                image_rect
            }
        };

        let atlas_scaling = if atlas_rect.is_some() || image.rect.is_some() {
            let atlas_scaling = size / rect.size();
            rect.min *= atlas_scaling;
            rect.max *= atlas_scaling;
            Some(atlas_scaling)
        } else {
            None
        };

        extracted_uinodes
            .uinodes
            .entry(entity.into())
            .or_default()
            .insert(
                commands.spawn_empty().id(),
                ExtractedUiNode {
                    z_order: stack_index.0 as f32 + stack_z_offsets::IMAGE,
                    clip: clip.map(|clip| clip.clip),
                    image: image.image.id(),
                    extracted_camera_entity,
                    transform: Affine2::from(*transform)
                        * Affine2::from_translation(visual_box.center()),
                    item: ExtractedUiItem::Node {
                        color: image.color.into(),
                        rect,
                        atlas_scaling,
                        flip_x: image.flip_x,
                        flip_y: image.flip_y,
                        border: BorderRect::ZERO,
                        border_radius: uinode.border_radius,
                        node_type: NodeType::Rect,
                    },
                },
            );
    }
}

pub fn extract_uinode_borders(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            Option<&Node>,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            AnyOf<(&BorderColor, &Outline)>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let image = AssetId::<Image>::default();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        node,
        computed_node,
        stack_index,
        transform,
        inherited_visibility,
        maybe_clip,
        camera,
        (maybe_border_color, maybe_outline),
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip invisible borders and removed nodes
        if !inherited_visibility.get() || node.is_some_and(|node| node.display == Display::None) {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        // Don't extract borders with zero width along all edges
        if computed_node.border() != BorderRect::ZERO
            && let Some(border_color) = maybe_border_color
        {
            let border_colors = [
                border_color.left.to_linear(),
                border_color.top.to_linear(),
                border_color.right.to_linear(),
                border_color.bottom.to_linear(),
            ];

            const BORDER_FLAGS: [u32; 4] = [
                shader_flags::BORDER_LEFT,
                shader_flags::BORDER_TOP,
                shader_flags::BORDER_RIGHT,
                shader_flags::BORDER_BOTTOM,
            ];
            let mut completed_flags = 0;

            for (i, &color) in border_colors.iter().enumerate() {
                if color.is_fully_transparent() {
                    continue;
                }

                let mut border_flags = BORDER_FLAGS[i];

                if completed_flags & border_flags != 0 {
                    continue;
                }

                for j in i + 1..4 {
                    if color == border_colors[j] {
                        border_flags |= BORDER_FLAGS[j];
                    }
                }
                completed_flags |= border_flags;

                let node = ExtractedUiNode {
                    z_order: stack_index.0 as f32 + stack_z_offsets::BORDER,
                    image,
                    clip: maybe_clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    transform: transform.into(),
                    item: ExtractedUiItem::Node {
                        color,
                        rect: Rect {
                            max: computed_node.size(),
                            ..Default::default()
                        },
                        atlas_scaling: None,
                        flip_x: false,
                        flip_y: false,
                        border: computed_node.border(),
                        border_radius: computed_node.border_radius(),
                        node_type: NodeType::Border(border_flags),
                    },
                };

                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(commands.spawn_empty().id(), node);
            }
        }

        if computed_node.outline_width() <= 0. {
            continue;
        }

        if let Some(outline) = maybe_outline.filter(|outline| !outline.color.is_fully_transparent())
        {
            let outline_size = computed_node.outlined_node_size();
            extracted_uinodes
                .uinodes
                .entry(entity.into())
                .or_default()
                .insert(
                    commands.spawn_empty().id(),
                    ExtractedUiNode {
                        z_order: stack_index.0 as f32 + stack_z_offsets::BORDER,
                        image,
                        clip: maybe_clip.map(|clip| clip.clip),
                        extracted_camera_entity,
                        transform: transform.into(),
                        item: ExtractedUiItem::Node {
                            color: outline.color.into(),
                            rect: Rect {
                                max: outline_size,
                                ..Default::default()
                            },
                            atlas_scaling: None,
                            flip_x: false,
                            flip_y: false,
                            border: BorderRect::all(computed_node.outline_width()),
                            border_radius: computed_node.outline_radius(),
                            node_type: NodeType::Border(shader_flags::BORDER_ALL),
                        },
                    },
                );
        }
    }
}

/// The UI camera is "moved back" by this many units (plus the [`UI_CAMERA_TRANSFORM_OFFSET`]) and also has a view
/// distance of this many units. This ensures that with a left-handed projection,
/// as UI elements are "stacked on top of each other", they are within the camera's view
/// and have room to grow.
// TODO: Consider computing this value at runtime based on the maximum z-value.
const UI_CAMERA_FAR: f32 = 1000.0;

// This value is subtracted from the far distance for the camera's z-position to ensure nodes at z == 0.0 are rendered
// TODO: Evaluate if we still need this.
const UI_CAMERA_TRANSFORM_OFFSET: f32 = -0.1;

/// The ID of the subview associated with a camera on which UI is to be drawn.
///
/// When UI is present, cameras extract to two views: the main 2D/3D one and a
/// UI one. The main 2D or 3D camera gets subview 0, and the corresponding UI
/// camera gets this subview, 1.
const UI_CAMERA_SUBVIEW: u32 = 1;

/// A render-world component that lives on the main render target view and
/// specifies the corresponding UI view.
///
/// For example, if UI is being rendered to a 3D camera, this component lives on
/// the 3D camera and contains the entity corresponding to the UI view.
#[derive(Component)]
/// Entity id of the temporary render entity with the corresponding extracted UI view.
pub struct UiCameraView(pub Entity);

/// A render-world component that lives on the UI view and specifies the
/// corresponding main render target view.
///
/// For example, if the UI is being rendered to a 3D camera, this component
/// lives on the UI view and contains the entity corresponding to the 3D camera.
///
/// This is the inverse of [`UiCameraView`].
#[derive(Component)]
pub struct UiViewTarget(pub Entity);

/// Information that [`extract_ui_camera_view`] maintains about each view that
/// it has seen.
pub struct CachedUiViewData {
    /// The render-world [`ExtractedView`].
    extracted_view_entity: Entity,
    /// The unique, stable identifier for the view across frames.
    retained_view_entity: RetainedViewEntity,
}

/// Extracts all UI elements associated with a camera into the render world.
pub fn extract_ui_camera_view(
    mut commands: Commands,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    query: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                Ref<Camera>,
                Option<Ref<UiAntiAlias>>,
                Option<Ref<BoxShadowSamples>>,
            ),
            Or<(With<Camera2d>, With<Camera3d>)>,
        >,
    >,
    changed_query: Extract<
        Query<
            Entity,
            Or<(
                Changed<Camera>,
                Changed<UiAntiAlias>,
                Changed<BoxShadowSamples>,
                Changed<Camera2d>,
                Changed<Camera3d>,
            )>,
        >,
    >,
    main_pass_formats: Res<CameraMainPassTextureFormats>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
    mut cached_ui_view_data: Local<MainEntityHashMap<CachedUiViewData>>,
    (
        mut removed_cameras_query,
        mut removed_ui_anti_alias_query,
        mut removed_box_shadow_samples_query,
        mut removed_cameras_2d_query,
        mut removed_cameras_3d_query,
    ): (
        Extract<RemovedComponents<Camera>>,
        Extract<RemovedComponents<UiAntiAlias>>,
        Extract<RemovedComponents<BoxShadowSamples>>,
        Extract<RemovedComponents<Camera2d>>,
        Extract<RemovedComponents<Camera3d>>,
    ),
    mut changed_cameras: Local<MainEntityHashSet>,
    mut cameras_updated_this_frame: Local<MainEntityHashSet>,
) {
    changed_cameras.clear();
    for main_entity in changed_query
        .iter()
        .chain(removed_ui_anti_alias_query.read())
        .chain(removed_box_shadow_samples_query.read())
        .chain(removed_cameras_2d_query.read())
        .chain(removed_cameras_3d_query.read())
    {
        changed_cameras.insert(main_entity.into());
    }

    cameras_updated_this_frame.clear();
    for (main_entity, render_entity, camera, ui_anti_alias, shadow_samples) in &query {
        let main_entity = MainEntity::from(main_entity);
        let retained_view_entity = RetainedViewEntity::new(main_entity, None, UI_CAMERA_SUBVIEW);

        // ignore inactive cameras
        if let (Some(physical_viewport_rect), Some(target_size), Some(target_format)) = (
            camera.physical_viewport_rect(),
            camera.physical_target_size(),
            main_pass_formats.get(&render_entity).copied(),
        ) && target_size.x != 0
            && target_size.y != 0
            && camera.physical_viewport_size().is_some()
            && camera.is_active
        {
            cameras_updated_this_frame.insert(main_entity);
            transparent_render_phases.prepare_for_new_frame(retained_view_entity);

            // If the camera hasn't changed, we're done.
            if !changed_cameras.contains(&main_entity) {
                continue;
            }

            // use a projection matrix with the origin in the top left instead of the bottom left that comes with OrthographicProjection
            let projection_matrix = proj::orthographic(
                0.0,
                physical_viewport_rect.width() as f32,
                physical_viewport_rect.height() as f32,
                0.0,
                0.0,
                UI_CAMERA_FAR,
            );
            // We use `UI_CAMERA_SUBVIEW` here so as not to conflict with the
            // main 3D or 2D camera, which will have subview index 0.
            // Creates the UI view.
            let extracted_view = ExtractedView {
                retained_view_entity,
                clip_from_view: projection_matrix,
                world_from_view: GlobalTransform::from_xyz(
                    0.0,
                    0.0,
                    UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                ),
                clip_from_world: None,
                target_format,
                viewport: UVec4::from((physical_viewport_rect.min, physical_viewport_rect.size())),
                color_grading: Default::default(),
                invert_culling: false,
            };
            // Link to the main camera view.
            let ui_view_target_component = UiViewTarget(render_entity);

            let ui_camera_view = match cached_ui_view_data.get(&main_entity) {
                Some(cached_ui_view_data) => commands
                    .entity(cached_ui_view_data.extracted_view_entity)
                    .insert((extracted_view, ui_view_target_component))
                    .id(),
                None => commands
                    .spawn((extracted_view, ui_view_target_component))
                    .id(),
            };

            let mut entity_commands = commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.");
            // Link from the main 2D/3D camera view to the UI view.
            entity_commands.insert(UiCameraView(ui_camera_view));
            if let Some(ui_anti_alias) = ui_anti_alias {
                entity_commands.insert(*ui_anti_alias);
            }
            if let Some(shadow_samples) = shadow_samples {
                entity_commands.insert(*shadow_samples);
            }

            live_entities.insert(retained_view_entity);
            cached_ui_view_data.insert(
                main_entity,
                CachedUiViewData {
                    extracted_view_entity: ui_camera_view,
                    retained_view_entity,
                },
            );
            continue;
        }

        // If we got here, the camera no longer exists or is no longer
        // renderable. Remove its associated render-world data.
        commands
            .get_entity(render_entity)
            .expect("Camera entity wasn't synced.")
            .remove::<(UiCameraView, UiAntiAlias, BoxShadowSamples)>();
        live_entities.remove(&retained_view_entity);
        if let Some(cached_ui_view_data) = cached_ui_view_data.remove(&main_entity) {
            commands
                .entity(cached_ui_view_data.extracted_view_entity)
                .despawn();
        }
    }

    // Only remove the render-world data for a camera if we didn't handle the
    // camera above.
    // It's possible that the `Camera` component was removed and added in the
    // same frame.
    for main_entity in removed_cameras_query.read() {
        let main_entity = MainEntity::from(main_entity);
        if cameras_updated_this_frame.contains(&main_entity) {
            continue;
        }

        if let Some(cached_ui_view_data) = cached_ui_view_data.remove(&main_entity) {
            commands
                .entity(cached_ui_view_data.extracted_view_entity)
                .despawn();
            live_entities.remove(&cached_ui_view_data.retained_view_entity);
        }
    }

    // Clean up render phases belonging to cameras that no longer exist.
    transparent_render_phases.retain(|entity, _| live_entities.contains(entity));
}

pub fn extract_viewport_nodes(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(&Camera, &RenderTarget)>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ViewportNode,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        stack_index,
        transform,
        inherited_visibility,
        clip,
        camera,
        viewport_node,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip invisible images
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };
        let Some(camera_entity) = viewport_node.camera else {
            continue;
        };

        let Some(image) = camera_query
            .get(camera_entity)
            .ok()
            .and_then(|(_, render_target)| render_target.as_image())
        else {
            continue;
        };

        extracted_uinodes
            .uinodes
            .entry(entity.into())
            .or_default()
            .insert(
                commands.spawn_empty().id(),
                ExtractedUiNode {
                    z_order: stack_index.0 as f32 + stack_z_offsets::IMAGE,
                    clip: clip.map(|clip| clip.clip),
                    image: image.id(),
                    extracted_camera_entity,
                    transform: transform.into(),
                    item: ExtractedUiItem::Node {
                        color: LinearRgba::WHITE,
                        rect: Rect {
                            min: Vec2::ZERO,
                            max: uinode.size,
                        },
                        atlas_scaling: None,
                        flip_x: false,
                        flip_y: false,
                        border: uinode.border(),
                        border_radius: uinode.border_radius(),
                        node_type: NodeType::Rect,
                    },
                },
            );
    }
}

pub fn extract_text_sections(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ComputedTextBlock,
            &TextColor,
            &TextLayoutInfo,
            Option<&EditableText>,
            Option<&TextCursorStyle>,
        )>,
    >,
    text_styles: Extract<Query<&TextColor>>,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    let mut glyphs = vec![];

    for (
        entity,
        uinode,
        stack_index,
        global_transform,
        inherited_visibility,
        maybe_clip,
        camera,
        computed_block,
        text_color,
        text_layout_info,
        editable_text,
        cursor_style,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let transform = Affine2::from(*global_transform)
            * Affine2::from_translation(
                uinode.content_box().min
                    - editable_text.map_or(Vec2::ZERO, |text| text.viewport.offset),
            );

        let clip = if editable_text.is_some() {
            let content_box = uinode.content_box();
            let text_clip = Rect::from_center_size(
                global_transform.affine().translation + content_box.center(),
                content_box.size(),
            );
            Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)))
        } else {
            maybe_clip.map(|clip| clip.clip)
        };

        let mut color = text_color.0.to_linear();

        let selected_text_color = cursor_style
            .and_then(|cursor_style| cursor_style.selected_text_color)
            .map(|selected_text_color| selected_text_color.to_linear());

        let mut current_section_index = 0;

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                section_index,
                ..
            },
        ) in text_layout_info.glyphs.iter().enumerate()
        {
            if current_section_index != *section_index
                && let Some(section_entity) = computed_block
                    .entities()
                    .get(*section_index as usize)
                    .map(|t| t.entity)
            {
                color = text_styles
                    .get(section_entity)
                    .map(|text_color| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                current_section_index = *section_index;
            }

            let color = if !atlas_info.is_alpha_mask {
                LinearRgba::WHITE
            } else if let Some(selected_text_color) = selected_text_color
                && text_layout_info
                    .selection_rects
                    .iter()
                    .any(|selection_rect| {
                        let glyph_rect = Rect::from_center_size(*position, atlas_info.rect.size());
                        selection_rect.contains(glyph_rect.min)
                            && selection_rect.contains(glyph_rect.max)
                    })
            {
                selected_text_color
            } else {
                color
            };

            glyphs.push(ExtractedGlyph {
                color,
                translation: *position,
                rect: atlas_info.rect,
            });

            if text_layout_info
                .glyphs
                .get(i + 1)
                .is_none_or(|info| info.atlas_info.texture != atlas_info.texture)
            {
                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT,
                            image: atlas_info.texture,
                            clip,
                            extracted_camera_entity,
                            item: ExtractedUiItem::Glyphs {
                                glyphs: mem::take(&mut glyphs),
                            },
                            transform,
                        },
                    );
            }
        }
    }
}

pub fn extract_text_shadows(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &UiGlobalTransform,
            &ComputedUiTargetCamera,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &TextLayoutInfo,
            &TextShadow,
            &ComputedTextBlock,
            Option<&EditableText>,
        )>,
    >,
    text_decoration_query: Extract<Query<(Has<Strikethrough>, Has<Underline>)>>,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    let mut glyphs = vec![];

    for (
        entity,
        uinode,
        stack_index,
        global_transform,
        target,
        inherited_visibility,
        maybe_clip,
        text_layout_info,
        shadow,
        computed_block,
        editable_text,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target) else {
            continue;
        };

        let node_transform = Affine2::from(*global_transform)
            * Affine2::from_translation(
                uinode.content_box().min + shadow.offset / uinode.inverse_scale_factor()
                    - editable_text.map_or(Vec2::ZERO, |text| text.viewport.offset),
            );

        let clip = if editable_text.is_some() {
            let content_box = uinode.content_box();
            let text_clip = Rect::from_center_size(
                global_transform.affine().translation + content_box.center(),
                content_box.size(),
            );
            Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)))
        } else {
            maybe_clip.map(|clip| clip.clip)
        };

        for (
            i,
            PositionedGlyph {
                position,
                atlas_info,
                section_index,
                ..
            },
        ) in text_layout_info.glyphs.iter().enumerate()
        {
            glyphs.push(ExtractedGlyph {
                color: shadow.color.into(),
                translation: *position,
                rect: atlas_info.rect,
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.section_index != *section_index
                    || info.atlas_info.texture != atlas_info.texture
            }) {
                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            transform: node_transform,
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT,
                            image: atlas_info.texture,
                            clip,
                            extracted_camera_entity,
                            item: ExtractedUiItem::Glyphs {
                                glyphs: mem::take(&mut glyphs),
                            },
                        },
                    );
            }
        }

        for run in text_layout_info.run_geometry.iter() {
            let Some(section_entity) = computed_block
                .entities()
                .get(run.section_index as usize)
                .map(|t| t.entity)
            else {
                continue;
            };
            let Ok((has_strikethrough, has_underline)) = text_decoration_query.get(section_entity)
            else {
                continue;
            };

            if has_strikethrough {
                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: node_transform
                                * Affine2::from_translation(run.strikethrough_position()),
                            item: ExtractedUiItem::Node {
                                color: shadow.color.into(),
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: run.strikethrough_size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::ZERO,
                                border_radius: ResolvedBorderRadius::ZERO,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }

            if has_underline {
                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: node_transform
                                * Affine2::from_translation(run.underline_position()),
                            item: ExtractedUiItem::Node {
                                color: shadow.color.into(),
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: run.underline_size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::ZERO,
                                border_radius: ResolvedBorderRadius::ZERO,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }
        }
    }
}

pub fn extract_text_decorations(
    mut commands: Commands,
    extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedStackIndex,
            &ComputedTextBlock,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &TextLayoutInfo,
            Option<&EditableText>,
        )>,
    >,
    text_background_colors_query: Extract<
        Query<(
            AnyOf<(&TextBackgroundColor, &Strikethrough, &Underline)>,
            &TextColor,
            Option<&StrikethroughColor>,
            Option<&UnderlineColor>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let extracted_uinodes = extracted_uinodes.into_inner();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        stack_index,
        computed_block,
        global_transform,
        inherited_visibility,
        maybe_clip,
        camera,
        text_layout_info,
        editable_text,
    ) in extracted_uinodes
        .changed
        .iter()
        .flat_map(|main_entity| uinode_query.get(main_entity.entity()).ok())
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let transform = Affine2::from(global_transform)
            * Affine2::from_translation(
                uinode.content_box().min
                    - editable_text.map_or(Vec2::ZERO, |text| text.viewport.offset),
            );

        let clip = if editable_text.is_some() {
            let content_box = uinode.content_box();
            let text_clip = Rect::from_center_size(
                global_transform.affine().translation + content_box.center(),
                content_box.size(),
            );
            Some(maybe_clip.map_or(text_clip, |clip| clip.clip.intersect(text_clip)))
        } else {
            maybe_clip.map(|clip| clip.clip)
        };

        for run in text_layout_info.run_geometry.iter() {
            let Some(section_entity) = computed_block
                .entities()
                .get(run.section_index as usize)
                .map(|t| t.entity)
            else {
                continue;
            };
            let Ok((
                (text_background_color, maybe_strikethrough, maybe_underline),
                text_color,
                maybe_strikethrough_color,
                maybe_underline_color,
            )) = text_background_colors_query.get(section_entity)
            else {
                continue;
            };

            if let Some(text_background_color) = text_background_color {
                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: transform * Affine2::from_translation(run.bounds.center()),
                            item: ExtractedUiItem::Node {
                                color: text_background_color.0.to_linear(),
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: run.bounds.size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::ZERO,
                                border_radius: ResolvedBorderRadius::ZERO,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }

            if maybe_strikethrough.is_some() {
                let color = maybe_strikethrough_color
                    .map(|sc| sc.0)
                    .unwrap_or(text_color.0)
                    .to_linear();

                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT_STRIKETHROUGH,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: transform
                                * Affine2::from_translation(run.strikethrough_position()),
                            item: ExtractedUiItem::Node {
                                color,
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: run.strikethrough_size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::ZERO,
                                border_radius: ResolvedBorderRadius::ZERO,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }

            if maybe_underline.is_some() {
                let color = maybe_underline_color
                    .map(|uc| uc.0)
                    .unwrap_or(text_color.0)
                    .to_linear();

                extracted_uinodes
                    .uinodes
                    .entry(entity.into())
                    .or_default()
                    .insert(
                        commands.spawn_empty().id(),
                        ExtractedUiNode {
                            z_order: stack_index.0 as f32 + stack_z_offsets::TEXT_STRIKETHROUGH,
                            clip,
                            image: AssetId::default(),
                            extracted_camera_entity,
                            transform: transform
                                * Affine2::from_translation(run.underline_position()),
                            item: ExtractedUiItem::Node {
                                color,
                                rect: Rect {
                                    min: Vec2::ZERO,
                                    max: run.underline_size(),
                                },
                                atlas_scaling: None,
                                flip_x: false,
                                flip_y: false,
                                border: BorderRect::ZERO,
                                border_radius: ResolvedBorderRadius::ZERO,
                                node_type: NodeType::Rect,
                            },
                        },
                    );
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Shader flags to determine how to render the UI node.
    /// See [`shader_flags`] for possible values.
    pub flags: u32,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub radius: [[f32; 4]; 2],
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: [f32; 4],
    /// Size of the UI node.
    pub size: [f32; 2],
    /// Position relative to the center of the UI node.
    pub point: [f32; 2],
}

#[derive(Resource)]
pub struct UiMeta {
    vertices: RawBufferVec<UiVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

pub(crate) const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

pub(crate) const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component, Debug)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub image: AssetId<Image>,
}

/// The values here should match the values for the constants in `ui.wgsl`
pub mod shader_flags {
    /// Texture should be ignored
    pub const UNTEXTURED: u32 = 0;
    /// Textured
    pub const TEXTURED: u32 = 1;
    /// Ordering: top left, top right, bottom right, bottom left.
    pub const CORNERS: [u32; 4] = [0, 2, 2 | 4, 4];
    pub const RADIAL: u32 = 16;
    pub const FILL_START: u32 = 32;
    pub const FILL_END: u32 = 64;
    pub const CONIC: u32 = 128;
    pub const BORDER_LEFT: u32 = 256;
    pub const BORDER_TOP: u32 = 512;
    pub const BORDER_RIGHT: u32 = 1024;
    pub const BORDER_BOTTOM: u32 = 2048;
    pub const BORDER_ALL: u32 = BORDER_LEFT + BORDER_TOP + BORDER_RIGHT + BORDER_BOTTOM;
    pub const INVERT: u32 = 4096;
}

pub fn queue_uinodes(
    extracted_uinodes: Res<ExtractedUiNodes>,
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

    for (main_entity, extracted_sub_uinodes) in extracted_uinodes.uinodes.iter() {
        for (render_entity, extracted_uinode) in extracted_sub_uinodes.iter() {
            if current_camera_entity != extracted_uinode.extracted_camera_entity {
                current_phase = render_views
                    .get(extracted_uinode.extracted_camera_entity)
                    .ok()
                    .and_then(|(default_camera_view, ui_anti_alias)| {
                        camera_views
                            .get(default_camera_view.0)
                            .ok()
                            .and_then(|view| {
                                transparent_render_phases
                                    .get_mut(&view.retained_view_entity)
                                    .map(|transparent_phase| {
                                        (view, ui_anti_alias, transparent_phase)
                                    })
                            })
                    });
                current_camera_entity = extracted_uinode.extracted_camera_entity;
            }

            let Some((view, ui_anti_alias, transparent_phase)) = current_phase.as_mut() else {
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
                sort_key: FloatOrd(extracted_uinode.z_order),
                // batch_range will be calculated in prepare_uinodes
                batch_range: 0..0,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Resource, Default)]
pub struct ImageNodeBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

pub fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    mut ui_meta: ResMut<UiMeta>,
    extracted_uinodes: Res<ExtractedUiNodes>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiPipeline>,
    mut image_bind_groups: ResMut<ImageNodeBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            AssetEvent::Unused { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_view_bind_group",
            &pipeline_cache.get_bind_group_layout(&ui_pipeline.view_layout),
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_image_handle = None;

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                let Some(extracted_uinode) = extracted_uinodes
                    .uinodes
                    .get(&item.main_entity())
                    .and_then(|sub_uinodes| sub_uinodes.get(&item.entity()))
                else {
                    batch_image_handle = None;
                    continue;
                };

                // Initialize the batch range to be zero-length initially.
                // We'll extend it as we accumulate items into this batch.
                item.batch_range = (item_index as u32)..(item_index as u32);

                let mut existing_batch = batches.last_mut();

                if batch_image_handle.is_none()
                    || existing_batch.is_none()
                    || (batch_image_handle != Some(AssetId::default())
                        && extracted_uinode.image != AssetId::default()
                        && batch_image_handle != Some(extracted_uinode.image))
                {
                    if let Some(gpu_image) = gpu_images.get(extracted_uinode.image) {
                        batch_item_index = item_index;
                        batch_image_handle = Some(extracted_uinode.image);

                        let new_batch = UiBatch {
                            range: vertices_index..vertices_index,
                            image: extracted_uinode.image,
                        };
                        batches.push((item.entity(), new_batch));

                        image_bind_groups
                            .values
                            .entry(extracted_uinode.image)
                            .or_insert_with(|| {
                                render_device.create_bind_group(
                                    "ui_material_bind_group",
                                    &pipeline_cache
                                        .get_bind_group_layout(&ui_pipeline.image_layout),
                                    &BindGroupEntries::sequential((
                                        &gpu_image.texture_view,
                                        &gpu_image.sampler,
                                    )),
                                )
                            });

                        existing_batch = batches.last_mut();
                    } else {
                        continue;
                    }
                } else if batch_image_handle == Some(AssetId::default())
                    && extracted_uinode.image != AssetId::default()
                {
                    if let Some(ref mut existing_batch) = existing_batch
                        && let Some(gpu_image) = gpu_images.get(extracted_uinode.image)
                    {
                        batch_image_handle = Some(extracted_uinode.image);
                        existing_batch.1.image = extracted_uinode.image;

                        image_bind_groups
                            .values
                            .entry(extracted_uinode.image)
                            .or_insert_with(|| {
                                render_device.create_bind_group(
                                    "ui_material_bind_group",
                                    &pipeline_cache
                                        .get_bind_group_layout(&ui_pipeline.image_layout),
                                    &BindGroupEntries::sequential((
                                        &gpu_image.texture_view,
                                        &gpu_image.sampler,
                                    )),
                                )
                            });
                    } else {
                        continue;
                    }
                }
                match &extracted_uinode.item {
                    ExtractedUiItem::Node {
                        atlas_scaling,
                        flip_x,
                        flip_y,
                        border_radius,
                        border,
                        node_type,
                        rect,
                        color,
                    } => {
                        let mut flags = if extracted_uinode.image != AssetId::default() {
                            shader_flags::TEXTURED
                        } else {
                            shader_flags::UNTEXTURED
                        };

                        let mut uinode_rect = *rect;

                        let rect_size = uinode_rect.size();

                        let transform = extracted_uinode.transform;

                        // Specify the corners of the node
                        let positions = QUAD_VERTEX_POSITIONS
                            .map(|pos| transform.transform_point2(pos * rect_size).extend(0.));
                        let points = QUAD_VERTEX_POSITIONS.map(|pos| pos * rect_size);

                        // Calculate the effect of clipping
                        // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
                        let mut positions_diff = if let Some(clip) = extracted_uinode.clip {
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

                        let transformed_rect_size = transform.transform_vector2(rect_size).abs();

                        // Don't try to cull nodes that have a rotation
                        // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                        // In those two cases, the culling check can proceed normally as corners will be on
                        // horizontal / vertical lines
                        // For all other angles, bypass the culling check
                        // This does not properly handles all rotations on all axis
                        if transform.x_axis[1] == 0.0 {
                            // Cull nodes that are completely clipped
                            if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                                || positions_diff[1].y - positions_diff[2].y
                                    >= transformed_rect_size.y
                            {
                                continue;
                            }
                        }
                        let uvs = if flags == shader_flags::UNTEXTURED {
                            [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
                        } else {
                            let image = gpu_images
                                .get(extracted_uinode.image)
                                .expect("Image was checked during batching and should still exist");
                            // Rescale atlases. This is done here because we need texture data that might not be available in Extract.
                            let atlas_extent = atlas_scaling
                                .map(|scaling| image.size_2d().as_vec2() * scaling)
                                .unwrap_or(uinode_rect.max);
                            if *flip_x {
                                mem::swap(&mut uinode_rect.max.x, &mut uinode_rect.min.x);
                                positions_diff[0].x *= -1.;
                                positions_diff[1].x *= -1.;
                                positions_diff[2].x *= -1.;
                                positions_diff[3].x *= -1.;
                            }
                            if *flip_y {
                                mem::swap(&mut uinode_rect.max.y, &mut uinode_rect.min.y);
                                positions_diff[0].y *= -1.;
                                positions_diff[1].y *= -1.;
                                positions_diff[2].y *= -1.;
                                positions_diff[3].y *= -1.;
                            }
                            [
                                Vec2::new(
                                    uinode_rect.min.x + positions_diff[0].x,
                                    uinode_rect.min.y + positions_diff[0].y,
                                ),
                                Vec2::new(
                                    uinode_rect.max.x + positions_diff[1].x,
                                    uinode_rect.min.y + positions_diff[1].y,
                                ),
                                Vec2::new(
                                    uinode_rect.max.x + positions_diff[2].x,
                                    uinode_rect.max.y + positions_diff[2].y,
                                ),
                                Vec2::new(
                                    uinode_rect.min.x + positions_diff[3].x,
                                    uinode_rect.max.y + positions_diff[3].y,
                                ),
                            ]
                            .map(|pos| pos / atlas_extent)
                        };

                        let color = color.to_f32_array();
                        match *node_type {
                            NodeType::Border(border_flags) => {
                                flags |= border_flags;
                            }
                            NodeType::Inverted => {
                                flags |= INVERT;
                            }
                            _ => {}
                        }

                        for i in 0..4 {
                            let ui_vertex = UiVertex {
                                position: positions_clipped[i].into(),
                                uv: uvs[i].into(),
                                color,
                                flags: flags | shader_flags::CORNERS[i],
                                radius: (*border_radius).into(),
                                border: [
                                    border.min_inset.x,
                                    border.min_inset.y,
                                    border.max_inset.x,
                                    border.max_inset.y,
                                ],
                                size: rect_size.into(),
                                point: points[i].into(),
                            };
                            ui_meta.vertices.push(ui_vertex);
                        }

                        for &i in &QUAD_INDICES {
                            ui_meta.indices.push(indices_index + i as u32);
                        }

                        vertices_index += 6;
                        indices_index += 4;
                    }
                    ExtractedUiItem::Glyphs { glyphs } => {
                        let image = gpu_images
                            .get(extracted_uinode.image)
                            .expect("Image was checked during batching and should still exist");

                        let atlas_extent = image.size_2d().as_vec2();

                        for glyph in glyphs {
                            let color = glyph.color.to_f32_array();
                            let glyph_rect = glyph.rect;
                            let rect_size = glyph_rect.size();

                            // Specify the corners of the glyph
                            let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                                extracted_uinode
                                    .transform
                                    .transform_point2(glyph.translation + pos * glyph_rect.size())
                                    .extend(0.)
                            });

                            let positions_diff = if let Some(clip) = extracted_uinode.clip {
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

                            // cull nodes that are completely clipped
                            let transformed_rect_size = extracted_uinode
                                .transform
                                .transform_vector2(rect_size)
                                .abs();
                            // Don't try to cull glyphs that have a rotation.
                            if extracted_uinode.transform.x_axis[1] == 0.0
                                && (positions_diff[0].x - positions_diff[1].x
                                    >= transformed_rect_size.x
                                    || positions_diff[1].y - positions_diff[2].y
                                        >= transformed_rect_size.y)
                            {
                                continue;
                            }

                            let uvs = [
                                Vec2::new(
                                    glyph.rect.min.x + positions_diff[0].x,
                                    glyph.rect.min.y + positions_diff[0].y,
                                ),
                                Vec2::new(
                                    glyph.rect.max.x + positions_diff[1].x,
                                    glyph.rect.min.y + positions_diff[1].y,
                                ),
                                Vec2::new(
                                    glyph.rect.max.x + positions_diff[2].x,
                                    glyph.rect.max.y + positions_diff[2].y,
                                ),
                                Vec2::new(
                                    glyph.rect.min.x + positions_diff[3].x,
                                    glyph.rect.max.y + positions_diff[3].y,
                                ),
                            ]
                            .map(|pos| pos / atlas_extent);

                            for i in 0..4 {
                                ui_meta.vertices.push(UiVertex {
                                    position: positions_clipped[i].into(),
                                    uv: uvs[i].into(),
                                    color,
                                    flags: shader_flags::TEXTURED | shader_flags::CORNERS[i],
                                    radius: [[0.0; 4]; 2],
                                    border: [0.0; 4],
                                    size: rect_size.into(),
                                    point: [0.0; 2],
                                });
                            }

                            for &i in &QUAD_INDICES {
                                ui_meta.indices.push(indices_index + i as u32);
                            }

                            vertices_index += 6;
                            indices_index += 4;
                        }
                    }
                }
                existing_batch.unwrap().1.range.end = vertices_index;
                ui_phase.items[batch_item_index].batch_range_mut().end += 1;
            }
        }

        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.try_insert_batch(batches);
    }
}

/// A render-world system that removes all [`UiBatch`] components.
///
/// They're currently rebuilt from scratch every frame, so we have to remove
/// them.
///
/// This is run during the render cleanup phase.
pub fn clear_batches(mut commands: Commands, batches_query: Query<Entity, With<UiBatch>>) {
    for entity in &batches_query {
        commands.entity(entity).remove::<UiBatch>();
    }
}
