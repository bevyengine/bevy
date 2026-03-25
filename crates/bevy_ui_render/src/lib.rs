#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Provides rendering functionality for `bevy_ui`.

pub mod box_shadow;
mod color_space;
mod gradient;
mod pipeline;
mod render_pass;
mod text;
pub mod ui_material;
mod ui_material_pipeline;
pub mod ui_texture_slice_pipeline;

#[cfg(feature = "bevy_ui_debug")]
mod debug_overlay;

use bevy_camera::visibility::InheritedVisibility;
use bevy_camera::{Camera, Camera2d, Camera3d, Hdr, RenderTarget};
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_shader::load_shader_library;
use bevy_sprite_render::SpriteAssetEvents;
use bevy_ui::widget::{ImageNode, TextShadow, ViewportNode};
use bevy_ui::{
    BackgroundColor, BorderColor, CalculatedClip, ComputedNode, ComputedUiTargetCamera, Display,
    Node, OuterColor, Outline, ResolvedBorderRadius, UiGlobalTransform,
};

use bevy_app::prelude::*;
use bevy_asset::{AssetEvent, AssetId, Assets};
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_core_pipeline::schedule::{Core2d, Core2dSystems, Core3d, Core3dSystems};
use bevy_core_pipeline::upscaling::upscaling;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::SystemParam;
use bevy_image::{prelude::*, TRANSPARENT_IMAGE_HANDLE};
use bevy_math::{Affine2, FloatOrd, Mat4, Rect, UVec4, Vec2};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::{
        sort_phase_system, AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex,
        ViewSortedRenderPhases,
    },
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    sync_world::{MainEntity, RenderEntity, TemporaryRenderEntity},
    texture::GpuImage,
    view::{ExtractedView, RetainedViewEntity, ViewUniforms},
    Extract, ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_sprite::BorderRect;
#[cfg(feature = "bevy_ui_debug")]
pub use debug_overlay::{GlobalUiDebugOptions, UiDebugOptions};

use color_space::ColorSpacePlugin;
use gradient::GradientPlugin;

use bevy_platform::collections::{HashMap, HashSet};
use bevy_text::{
    ComputedTextBlock, PositionedGlyph, Strikethrough, StrikethroughColor, TextBackgroundColor,
    TextColor, TextLayoutInfo, Underline, UnderlineColor,
};
use bevy_transform::components::GlobalTransform;
use box_shadow::BoxShadowPlugin;
use bytemuck::{Pod, Zeroable};
use core::ops::Range;

pub use pipeline::*;
pub use render_pass::*;
pub use ui_material_pipeline::*;
use ui_texture_slice_pipeline::UiTextureSlicerPlugin;

use crate::shader_flags::INVERT;
use crate::text::extract_text_cursor;

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
    pub const TEXT: f32 = 0.06;
    pub const TEXT_STRIKETHROUGH: f32 = 0.07;
    pub const TEXT_SELECTION: f32 = 0.08;
    pub const TEXT_CURSOR: f32 = 0.085;
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderUiSystems {
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
                    RenderUiSystems::ExtractCameraViews,
                    RenderUiSystems::ExtractBoxShadows,
                    RenderUiSystems::ExtractBackgrounds,
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
                    extract_ui_camera_view.in_set(RenderUiSystems::ExtractCameraViews),
                    extract_uinode_background_colors.in_set(RenderUiSystems::ExtractBackgrounds),
                    extract_uinode_images.in_set(RenderUiSystems::ExtractImages),
                    extract_uinode_borders.in_set(RenderUiSystems::ExtractBorders),
                    extract_viewport_nodes.in_set(RenderUiSystems::ExtractViewportNodes),
                    extract_text_decorations.in_set(RenderUiSystems::ExtractTextBackgrounds),
                    extract_text_shadows.in_set(RenderUiSystems::ExtractTextShadows),
                    extract_text_sections.in_set(RenderUiSystems::ExtractText),
                    extract_text_cursor.in_set(RenderUiSystems::ExtractCursor),
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
        app.add_plugins(ColorSpacePlugin);
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
    pub main_entity: MainEntity,
    pub render_entity: Entity,
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
        /// Indices into [`ExtractedUiNodes::glyphs`]
        range: Range<usize>,
    },
}

pub struct ExtractedGlyph {
    pub color: LinearRgba,
    pub translation: Vec2,
    pub rect: Rect,
}

#[derive(Resource, Default)]
pub struct ExtractedUiNodes {
    pub uinodes: Vec<ExtractedUiNode>,
    pub glyphs: Vec<ExtractedGlyph>,
}

impl ExtractedUiNodes {
    pub fn clear(&mut self) {
        self.uinodes.clear();
        self.glyphs.clear();
    }
}

pub fn extract_uinode_background_colors(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
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
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        uinode,
        transform,
        inherited_visibility,
        clip,
        camera,
        background_color,
        maybe_outer_color,
    ) in &uinode_query
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
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offsets::BACKGROUND_COLOR,
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
                main_entity: entity.into(),
            });
        }

        if let Some(outer_color) = maybe_outer_color
            && !outer_color.0.is_fully_transparent()
        {
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
                z_order: uinode.stack_index as f32 + stack_z_offsets::BACKGROUND_COLOR,
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
                main_entity: entity.into(),
            });
        }
    }
}

pub fn extract_uinode_images(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ImageNode,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();
    for (entity, uinode, transform, inherited_visibility, clip, camera, image) in &uinode_query {
        let content_box = uinode.content_box();
        // Skip invisible images
        if !inherited_visibility.get()
            || image.color.is_fully_transparent()
            || image.image.id() == TRANSPARENT_IMAGE_HANDLE.id()
            || image.image_mode.uses_slices()
            || content_box.size().cmple(Vec2::ZERO).any()
        {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let atlas_rect = image
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(&texture_atlases))
            .map(|r| r.as_rect());

        let mut rect = match (atlas_rect, image.rect) {
            (None, None) => Rect {
                min: Vec2::ZERO,
                max: content_box.size(),
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
            let atlas_scaling = content_box.size() / rect.size();
            rect.min *= atlas_scaling;
            rect.max *= atlas_scaling;
            Some(atlas_scaling)
        } else {
            None
        };

        extracted_uinodes.uinodes.push(ExtractedUiNode {
            z_order: uinode.stack_index as f32 + stack_z_offsets::IMAGE,
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
            clip: clip.map(|clip| clip.clip),
            image: image.image.id(),
            extracted_camera_entity,
            transform: Affine2::from(*transform) * Affine2::from_translation(content_box.center()),
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
            main_entity: entity.into(),
        });
    }
}

pub fn extract_uinode_borders(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &Node,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            AnyOf<(&BorderColor, &Outline)>,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let image = AssetId::<Image>::default();
    let mut camera_mapper = camera_map.get_mapper();

    for (
        entity,
        node,
        computed_node,
        transform,
        inherited_visibility,
        maybe_clip,
        camera,
        (maybe_border_color, maybe_outline),
    ) in &uinode_query
    {
        // Skip invisible borders and removed nodes
        if !inherited_visibility.get() || node.display == Display::None {
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

                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: computed_node.stack_index as f32 + stack_z_offsets::BORDER,
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
                    main_entity: entity.into(),
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                });
            }
        }

        if computed_node.outline_width() <= 0. {
            continue;
        }

        if let Some(outline) = maybe_outline.filter(|outline| !outline.color.is_fully_transparent())
        {
            let outline_size = computed_node.outlined_node_size();
            extracted_uinodes.uinodes.push(ExtractedUiNode {
                z_order: computed_node.stack_index as f32 + stack_z_offsets::BORDER,
                render_entity: commands.spawn(TemporaryRenderEntity).id(),
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
                main_entity: entity.into(),
            });
        }
    }
}

/// The UI camera is "moved back" by this many units (plus the [`UI_CAMERA_TRANSFORM_OFFSET`]) and also has a view
/// distance of this many units. This ensures that with a left-handed projection,
/// as ui elements are "stacked on top of each other", they are within the camera's view
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

/// Extracts all UI elements associated with a camera into the render world.
pub fn extract_ui_camera_view(
    mut commands: Commands,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    query: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &Camera,
                Has<Hdr>,
                Option<&UiAntiAlias>,
                Option<&BoxShadowSamples>,
            ),
            Or<(With<Camera2d>, With<Camera3d>)>,
        >,
    >,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
) {
    live_entities.clear();

    for (main_entity, render_entity, camera, hdr, ui_anti_alias, shadow_samples) in &query {
        // ignore inactive cameras
        if !camera.is_active {
            commands
                .get_entity(render_entity)
                .expect("Camera entity wasn't synced.")
                .remove::<(UiCameraView, UiAntiAlias, BoxShadowSamples)>();
            continue;
        }

        if let Some(physical_viewport_rect) = camera.physical_viewport_rect() {
            // use a projection matrix with the origin in the top left instead of the bottom left that comes with OrthographicProjection
            let projection_matrix = Mat4::orthographic_rh(
                0.0,
                physical_viewport_rect.width() as f32,
                physical_viewport_rect.height() as f32,
                0.0,
                0.0,
                UI_CAMERA_FAR,
            );
            // We use `UI_CAMERA_SUBVIEW` here so as not to conflict with the
            // main 3D or 2D camera, which will have subview index 0.
            let retained_view_entity =
                RetainedViewEntity::new(main_entity.into(), None, UI_CAMERA_SUBVIEW);
            // Creates the UI view.
            let ui_camera_view = commands
                .spawn((
                    ExtractedView {
                        retained_view_entity,
                        clip_from_view: projection_matrix,
                        world_from_view: GlobalTransform::from_xyz(
                            0.0,
                            0.0,
                            UI_CAMERA_FAR + UI_CAMERA_TRANSFORM_OFFSET,
                        ),
                        clip_from_world: None,
                        hdr,
                        viewport: UVec4::from((
                            physical_viewport_rect.min,
                            physical_viewport_rect.size(),
                        )),
                        color_grading: Default::default(),
                        invert_culling: false,
                    },
                    // Link to the main camera view.
                    UiViewTarget(render_entity),
                    TemporaryRenderEntity,
                ))
                .id();

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
            transparent_render_phases.prepare_for_new_frame(retained_view_entity);

            live_entities.insert(retained_view_entity);
        }
    }

    transparent_render_phases.retain(|entity, _| live_entities.contains(entity));
}

pub fn extract_viewport_nodes(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    camera_query: Extract<Query<(&Camera, &RenderTarget)>>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ViewportNode,
        )>,
    >,
    camera_map: Extract<UiCameraMap>,
) {
    let mut camera_mapper = camera_map.get_mapper();
    for (entity, uinode, transform, inherited_visibility, clip, camera, viewport_node) in
        &uinode_query
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

        extracted_uinodes.uinodes.push(ExtractedUiNode {
            z_order: uinode.stack_index as f32 + stack_z_offsets::IMAGE,
            render_entity: commands.spawn(TemporaryRenderEntity).id(),
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
            main_entity: entity.into(),
        });
    }
}

pub fn extract_text_sections(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &ComputedTextBlock,
            &TextColor,
            &TextLayoutInfo,
        )>,
    >,
    text_styles: Extract<Query<&TextColor>>,
    camera_map: Extract<UiCameraMap>,
) {
    let mut start = extracted_uinodes.glyphs.len();
    let mut end = start + 1;

    let mut camera_mapper = camera_map.get_mapper();
    for (
        entity,
        uinode,
        transform,
        inherited_visibility,
        clip,
        camera,
        computed_block,
        text_color,
        text_layout_info,
    ) in &uinode_query
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let transform =
            Affine2::from(*transform) * Affine2::from_translation(uinode.content_box().min);

        let mut color = text_color.0.to_linear();

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
                    .get(*section_index)
                    .map(|t| t.entity)
            {
                color = text_styles
                    .get(section_entity)
                    .map(|text_color| LinearRgba::from(text_color.0))
                    .unwrap_or_default();
                current_section_index = *section_index;
            }

            extracted_uinodes.glyphs.push(ExtractedGlyph {
                color,
                translation: *position,
                rect: atlas_info.rect,
            });

            if text_layout_info
                .glyphs
                .get(i + 1)
                .is_none_or(|info| info.atlas_info.texture != atlas_info.texture)
            {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    image: atlas_info.texture,
                    clip: clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    item: ExtractedUiItem::Glyphs { range: start..end },
                    main_entity: entity.into(),
                    transform,
                });
                start = end;
            }

            end += 1;
        }
    }
}

pub fn extract_text_shadows(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &UiGlobalTransform,
            &ComputedUiTargetCamera,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &TextLayoutInfo,
            &TextShadow,
            &ComputedTextBlock,
        )>,
    >,
    text_decoration_query: Extract<Query<(Has<Strikethrough>, Has<Underline>)>>,
    camera_map: Extract<UiCameraMap>,
) {
    let mut start = extracted_uinodes.glyphs.len();
    let mut end = start + 1;

    let mut camera_mapper = camera_map.get_mapper();
    for (
        entity,
        uinode,
        transform,
        target,
        inherited_visibility,
        clip,
        text_layout_info,
        shadow,
        computed_block,
    ) in &uinode_query
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(target) else {
            continue;
        };

        let node_transform = Affine2::from(*transform)
            * Affine2::from_translation(
                uinode.content_box().min + shadow.offset / uinode.inverse_scale_factor(),
            );

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
            extracted_uinodes.glyphs.push(ExtractedGlyph {
                color: shadow.color.into(),
                translation: *position,
                rect: atlas_info.rect,
            });

            if text_layout_info.glyphs.get(i + 1).is_none_or(|info| {
                info.section_index != *section_index
                    || info.atlas_info.texture != atlas_info.texture
            }) {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    transform: node_transform,
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    image: atlas_info.texture,
                    clip: clip.map(|clip| clip.clip),
                    extracted_camera_entity,
                    item: ExtractedUiItem::Glyphs { range: start..end },
                    main_entity: entity.into(),
                });
                start = end;
            }

            end += 1;
        }

        for run in text_layout_info.run_geometry.iter() {
            let Some(section_entity) = computed_block
                .entities()
                .get(run.section_index)
                .map(|t| t.entity)
            else {
                continue;
            };
            let Ok((has_strikethrough, has_underline)) = text_decoration_query.get(section_entity)
            else {
                continue;
            };

            if has_strikethrough {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    clip: clip.map(|clip| clip.clip),
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
                    main_entity: entity.into(),
                });
            }

            if has_underline {
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    clip: clip.map(|clip| clip.clip),
                    image: AssetId::default(),
                    extracted_camera_entity,
                    transform: node_transform * Affine2::from_translation(run.underline_position()),
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
                    main_entity: entity.into(),
                });
            }
        }
    }
}

pub fn extract_text_decorations(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
    uinode_query: Extract<
        Query<(
            Entity,
            &ComputedNode,
            &ComputedTextBlock,
            &UiGlobalTransform,
            &InheritedVisibility,
            Option<&CalculatedClip>,
            &ComputedUiTargetCamera,
            &TextLayoutInfo,
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
    let mut camera_mapper = camera_map.get_mapper();
    for (
        entity,
        uinode,
        computed_block,
        global_transform,
        inherited_visibility,
        clip,
        camera,
        text_layout_info,
    ) in &uinode_query
    {
        // Skip if not visible or if size is set to zero (e.g. when a parent is set to `Display::None`)
        if !inherited_visibility.get() || uinode.is_empty() {
            continue;
        }

        let Some(extracted_camera_entity) = camera_mapper.map(camera) else {
            continue;
        };

        let transform =
            Affine2::from(global_transform) * Affine2::from_translation(uinode.content_box().min);

        for run in text_layout_info.run_geometry.iter() {
            let Some(section_entity) = computed_block
                .entities()
                .get(run.section_index)
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
                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    clip: clip.map(|clip| clip.clip),
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
                    main_entity: entity.into(),
                });
            }

            if maybe_strikethrough.is_some() {
                let color = maybe_strikethrough_color
                    .map(|sc| sc.0)
                    .unwrap_or(text_color.0)
                    .to_linear();

                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT_STRIKETHROUGH,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    clip: clip.map(|clip| clip.clip),
                    image: AssetId::default(),
                    extracted_camera_entity,
                    transform: transform * Affine2::from_translation(run.strikethrough_position()),
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
                    main_entity: entity.into(),
                });
            }

            if maybe_underline.is_some() {
                let color = maybe_underline_color
                    .map(|uc| uc.0)
                    .unwrap_or(text_color.0)
                    .to_linear();

                extracted_uinodes.uinodes.push(ExtractedUiNode {
                    z_order: uinode.stack_index as f32 + stack_z_offsets::TEXT_STRIKETHROUGH,
                    render_entity: commands.spawn(TemporaryRenderEntity).id(),
                    clip: clip.map(|clip| clip.clip),
                    image: AssetId::default(),
                    extracted_camera_entity,
                    transform: transform * Affine2::from_translation(run.underline_position()),
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
                    main_entity: entity.into(),
                });
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    /// Shader flags to determine how to render the UI node.
    /// See [`shader_flags`] for possible values.
    pub flags: u32,
    /// Border radius of the UI node.
    /// Ordering: top left, top right, bottom right, bottom left.
    pub radius: [f32; 4],
    /// Border thickness of the UI node.
    /// Ordering: left, top, right, bottom.
    pub border: [f32; 4],
    /// Size of the UI node.
    pub size: [f32; 2],
    /// Position relative to the center of the UI node.
    pub point: [f32; 2],
    /// Clip rect in screen space (min_x, min_y, max_x, max_y).
    /// For non-rotated nodes, vertex clipping is used and this is set to infinity.
    /// For rotated nodes, fragment-level clipping is used via this field.
    pub clip: [f32; 4],
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

#[derive(Component)]
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

    for (index, extracted_uinode) in extracted_uinodes.uinodes.iter().enumerate() {
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
                                .map(|transparent_phase| (view, ui_anti_alias, transparent_phase))
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
                hdr: view.hdr,
                anti_alias: matches!(ui_anti_alias, None | Some(UiAntiAlias::On)),
            },
        );

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline,
            entity: (extracted_uinode.render_entity, extracted_uinode.main_entity),
            sort_key: FloatOrd(extracted_uinode.z_order),
            index,
            // batch_range will be calculated in prepare_uinodes
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::None,
            indexed: true,
        });
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
    mut extracted_uinodes: ResMut<ExtractedUiNodes>,
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
                    .get(item.index)
                    .filter(|n| item.entity() == n.render_entity)
                else {
                    batch_image_handle = None;
                    continue;
                };

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

                        // Calculate the effect of clipping.
                        // For rotated nodes, vertex clipping is incorrect (corners are not
                        // axis-aligned), so we skip it and use fragment-level clipping via
                        // the `clip` vertex attribute and a discard in the shader instead.
                        let is_rotated = transform.x_axis[1] != 0.0;
                        let positions_diff = if !is_rotated {
                            if let Some(clip) = extracted_uinode.clip {
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
                            }
                        } else {
                            // Rotated: leave vertices unmodified; clipping is handled in shader.
                            [Vec2::ZERO; 4]
                        };
                        // Shader-level clip rect, used only for rotated nodes.
                        // For non-rotated nodes, vertex clipping is already correct, so we
                        // disable the shader clip by setting it to infinite bounds.
                        let shader_clip = if is_rotated {
                            if let Some(clip) = extracted_uinode.clip {
                                [clip.min.x, clip.min.y, clip.max.x, clip.max.y]
                            } else {
                                [
                                    f32::NEG_INFINITY,
                                    f32::NEG_INFINITY,
                                    f32::INFINITY,
                                    f32::INFINITY,
                                ]
                            }
                        } else {
                            [
                                f32::NEG_INFINITY,
                                f32::NEG_INFINITY,
                                f32::INFINITY,
                                f32::INFINITY,
                            ]
                        };

                        let positions_clipped = [
                            positions[0] + positions_diff[0].extend(0.),
                            positions[1] + positions_diff[1].extend(0.),
                            positions[2] + positions_diff[2].extend(0.),
                            positions[3] + positions_diff[3].extend(0.),
                        ];

                        // Convert the screen-space clipping deltas to local (unscaled) space for use
                        // in UV and SDF point calculations. `positions_diff` is in world/screen space
                        // but UVs and the `point` SDF attribute must be in the node's local space.
                        // Without this conversion, UiTransform scale causes inverted image scaling
                        // and incorrect border radius rendering on clipped nodes.
                        let local_positions_diff =
                            if transform.matrix2.determinant().abs() > f32::EPSILON {
                                let inv = transform.matrix2.inverse();
                                positions_diff.map(|d| inv * d)
                            } else {
                                positions_diff
                            };

                        let points = [
                            points[0] + local_positions_diff[0],
                            points[1] + local_positions_diff[1],
                            points[2] + local_positions_diff[2],
                            points[3] + local_positions_diff[3],
                        ];

                        let transformed_rect_size = transform.transform_vector2(rect_size).abs();

                        // Don't try to cull nodes that have a rotation via vertex clipping diffs,
                        // since positions_diff is zeroed out for rotated nodes.
                        // Fragment-level clipping handles the rotated case in the shader.
                        if !is_rotated {
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
                            let mut local_uv_diff = local_positions_diff;
                            if *flip_x {
                                core::mem::swap(&mut uinode_rect.max.x, &mut uinode_rect.min.x);
                                local_uv_diff[0].x *= -1.;
                                local_uv_diff[1].x *= -1.;
                                local_uv_diff[2].x *= -1.;
                                local_uv_diff[3].x *= -1.;
                            }
                            if *flip_y {
                                core::mem::swap(&mut uinode_rect.max.y, &mut uinode_rect.min.y);
                                local_uv_diff[0].y *= -1.;
                                local_uv_diff[1].y *= -1.;
                                local_uv_diff[2].y *= -1.;
                                local_uv_diff[3].y *= -1.;
                            }
                            [
                                Vec2::new(
                                    uinode_rect.min.x + local_uv_diff[0].x,
                                    uinode_rect.min.y + local_uv_diff[0].y,
                                ),
                                Vec2::new(
                                    uinode_rect.max.x + local_uv_diff[1].x,
                                    uinode_rect.min.y + local_uv_diff[1].y,
                                ),
                                Vec2::new(
                                    uinode_rect.max.x + local_uv_diff[2].x,
                                    uinode_rect.max.y + local_uv_diff[2].y,
                                ),
                                Vec2::new(
                                    uinode_rect.min.x + local_uv_diff[3].x,
                                    uinode_rect.max.y + local_uv_diff[3].y,
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
                            ui_meta.vertices.push(UiVertex {
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
                                clip: shader_clip,
                            });
                        }

                        for &i in &QUAD_INDICES {
                            ui_meta.indices.push(indices_index + i as u32);
                        }

                        vertices_index += 6;
                        indices_index += 4;
                    }
                    ExtractedUiItem::Glyphs { range } => {
                        let image = gpu_images
                            .get(extracted_uinode.image)
                            .expect("Image was checked during batching and should still exist");

                        let atlas_extent = image.size_2d().as_vec2();

                        for glyph in &extracted_uinodes.glyphs[range.clone()] {
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

                            let is_rotated = extracted_uinode.transform.x_axis[1] != 0.0;
                            let positions_diff = if !is_rotated {
                                if let Some(clip) = extracted_uinode.clip {
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
                                }
                            } else {
                                [Vec2::ZERO; 4]
                            };
                            let shader_clip = if is_rotated {
                                if let Some(clip) = extracted_uinode.clip {
                                    [clip.min.x, clip.min.y, clip.max.x, clip.max.y]
                                } else {
                                    [
                                        f32::NEG_INFINITY,
                                        f32::NEG_INFINITY,
                                        f32::INFINITY,
                                        f32::INFINITY,
                                    ]
                                }
                            } else {
                                [
                                    f32::NEG_INFINITY,
                                    f32::NEG_INFINITY,
                                    f32::INFINITY,
                                    f32::INFINITY,
                                ]
                            };

                            let positions_clipped = [
                                positions[0] + positions_diff[0].extend(0.),
                                positions[1] + positions_diff[1].extend(0.),
                                positions[2] + positions_diff[2].extend(0.),
                                positions[3] + positions_diff[3].extend(0.),
                            ];

                            // Cull glyphs that are completely clipped (only valid for non-rotated).
                            let transformed_rect_size = extracted_uinode
                                .transform
                                .transform_vector2(rect_size)
                                .abs();
                            if !is_rotated
                                && (positions_diff[0].x - positions_diff[1].x
                                    >= transformed_rect_size.x
                                    || positions_diff[1].y - positions_diff[2].y
                                        >= transformed_rect_size.y)
                            {
                                continue;
                            }

                            // Convert screen-space clipping deltas to local (atlas) space for correct
                            // UV computation when the node has a UiTransform scale applied.
                            let local_positions_diff = if extracted_uinode
                                .transform
                                .matrix2
                                .determinant()
                                .abs()
                                > f32::EPSILON
                            {
                                let inv = extracted_uinode.transform.matrix2.inverse();
                                positions_diff.map(|d| inv * d)
                            } else {
                                positions_diff
                            };

                            let uvs = [
                                Vec2::new(
                                    glyph.rect.min.x + local_positions_diff[0].x,
                                    glyph.rect.min.y + local_positions_diff[0].y,
                                ),
                                Vec2::new(
                                    glyph.rect.max.x + local_positions_diff[1].x,
                                    glyph.rect.min.y + local_positions_diff[1].y,
                                ),
                                Vec2::new(
                                    glyph.rect.max.x + local_positions_diff[2].x,
                                    glyph.rect.max.y + local_positions_diff[2].y,
                                ),
                                Vec2::new(
                                    glyph.rect.min.x + local_positions_diff[3].x,
                                    glyph.rect.max.y + local_positions_diff[3].y,
                                ),
                            ]
                            .map(|pos| pos / atlas_extent);

                            for i in 0..4 {
                                ui_meta.vertices.push(UiVertex {
                                    position: positions_clipped[i].into(),
                                    uv: uvs[i].into(),
                                    color,
                                    flags: shader_flags::TEXTURED | shader_flags::CORNERS[i],
                                    radius: [0.0; 4],
                                    border: [0.0; 4],
                                    size: rect_size.into(),
                                    point: [0.0; 2],
                                    clip: shader_clip,
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
    extracted_uinodes.clear();
}

// ---------------------------------------------------------------------------
// Helper: replicates the UV-clipping math used in `prepare_uinodes` so it can
// be unit-tested without a GPU.
//
// Returns the four UV coordinates (TL, TR, BR, BL) for a node quad that is
// clipped to `clip` given the node's world-space `transform`, its
// `node_rect` (in local/layout pixels) and the atlas extent used to
// normalise UVs.
#[cfg(test)]
fn compute_clipped_uvs(
    transform: Affine2,
    node_rect: bevy_math::Rect,
    clip: bevy_math::Rect,
    atlas_extent: Vec2,
) -> [Vec2; 4] {
    let rect_size = node_rect.size();

    // Screen-space corner positions.
    let positions = QUAD_VERTEX_POSITIONS
        .map(|pos| transform.transform_point2(pos * rect_size));

    // Screen-space clipping deltas (same logic as `prepare_uinodes`).
    let positions_diff = [
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
    ];

    // Convert to local space (the fix).
    let local_positions_diff = if transform.matrix2.determinant().abs() > f32::EPSILON {
        let inv = transform.matrix2.inverse();
        positions_diff.map(|d| inv * d)
    } else {
        positions_diff
    };

    let uinode_rect = node_rect;
    [
        Vec2::new(
            uinode_rect.min.x + local_positions_diff[0].x,
            uinode_rect.min.y + local_positions_diff[0].y,
        ),
        Vec2::new(
            uinode_rect.max.x + local_positions_diff[1].x,
            uinode_rect.min.y + local_positions_diff[1].y,
        ),
        Vec2::new(
            uinode_rect.max.x + local_positions_diff[2].x,
            uinode_rect.max.y + local_positions_diff[2].y,
        ),
        Vec2::new(
            uinode_rect.min.x + local_positions_diff[3].x,
            uinode_rect.max.y + local_positions_diff[3].y,
        ),
    ]
    .map(|pos| pos / atlas_extent)
}

// ---------------------------------------------------------------------------
// Helper: same as `compute_clipped_uvs` but intentionally uses the OLD
// (unfixed) logic — `positions_diff` applied directly to UVs without
// converting to local space.  Used to confirm that the old code produced
// the wrong results, giving the tests a clear before/after comparison.
#[cfg(test)]
fn compute_clipped_uvs_broken(
    transform: Affine2,
    node_rect: bevy_math::Rect,
    clip: bevy_math::Rect,
    atlas_extent: Vec2,
) -> [Vec2; 4] {
    let rect_size = node_rect.size();
    let positions = QUAD_VERTEX_POSITIONS
        .map(|pos| transform.transform_point2(pos * rect_size));

    let positions_diff = [
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
    ];

    let uinode_rect = node_rect;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{Affine2, Mat2, Rect, Vec2};

    const EPS: f32 = 1e-4;

    fn approx_eq(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < EPS
    }

    // Build a simple scale-only Affine2 with the node centred at `center`.
    fn scale_transform(center: Vec2, scale: f32) -> Affine2 {
        Affine2 {
            matrix2: Mat2::from_diagonal(Vec2::splat(scale)),
            translation: center,
        }
    }

    // -----------------------------------------------------------------------
    // With scale = 1 (no UiTransform scale) the fixed code should produce
    // the same UVs as the old code.  This is a regression guard.
    #[test]
    fn uv_clipping_scale_one_unchanged() {
        // 100×100 node centred at (150, 100). Clip cuts the right 25 px.
        let node_rect = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100., 100.),
        };
        let clip = Rect {
            min: Vec2::new(100., 50.),
            max: Vec2::new(175., 150.), // right edge at 175, node TL at 100 so 75 px visible
        };
        let center = Vec2::new(150., 100.);
        let transform = scale_transform(center, 1.0);
        let atlas_extent = Vec2::new(100., 100.);

        let fixed = compute_clipped_uvs(transform, node_rect, clip, atlas_extent);
        let broken = compute_clipped_uvs_broken(transform, node_rect, clip, atlas_extent);

        // At scale=1 both implementations must agree.
        for i in 0..4 {
            assert!(
                approx_eq(fixed[i], broken[i]),
                "scale=1: corner {i} diverged: fixed={:?} broken={:?}",
                fixed[i],
                broken[i]
            );
        }
    }

    // -----------------------------------------------------------------------
    // The original bug: with scale=3 the old code mapped the right-clipped
    // TR corner to UV.x ≈ 0 (collapsing the visible texture to nothing),
    // while the fixed code correctly maps it to UV.x ≈ 2/3.
    //
    // Setup:
    //   node layout size = 100×100, UiTransform.scale = 3
    //   → visual (screen) size = 300×300
    //   node centre at (150, 150)
    //   → screen corners: TL=(0,0) TR=(300,0) BR=(300,300) BL=(0,300)
    //   clip rect = (0,0)→(200,200)  (cuts 100 screen-px off right & bottom)
    //
    // The visible fraction of the texture along each axis is 200/300 = 2/3,
    // so the clipped TR corner should sample UV.x = 2/3.
    #[test]
    fn uv_clipping_scale_three_right_and_bottom() {
        let node_rect = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100., 100.),
        };
        let clip = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(200., 200.),
        };
        let center = Vec2::new(150., 150.);
        let transform = scale_transform(center, 3.0);
        let atlas_extent = Vec2::new(100., 100.);

        let fixed = compute_clipped_uvs(transform, node_rect, clip, atlas_extent);
        let broken = compute_clipped_uvs_broken(transform, node_rect, clip, atlas_extent);

        let expected_tr_uv = Vec2::new(2. / 3., 0.);   // TR: x clipped to 2/3
        let expected_br_uv = Vec2::new(2. / 3., 2. / 3.); // BR: both axes clipped

        // Fixed code should be correct.
        assert!(
            approx_eq(fixed[1], expected_tr_uv),
            "fixed TR UV wrong: got {:?}, expected {:?}",
            fixed[1],
            expected_tr_uv
        );
        assert!(
            approx_eq(fixed[2], expected_br_uv),
            "fixed BR UV wrong: got {:?}, expected {:?}",
            fixed[2],
            expected_br_uv
        );

        // Old (broken) code should NOT produce the correct result — confirm
        // the test would have caught the bug.
        assert!(
            !approx_eq(broken[1], expected_tr_uv),
            "broken code accidentally produced correct TR UV — test may be invalid"
        );
    }

    // -----------------------------------------------------------------------
    // With scale=0.5 the visual node is half size.  Clipping from the left
    // by 10 screen pixels should cut 20 local pixels (10 / 0.5) off the TL
    // UV, i.e. TL UV.x = 20/100 = 0.2.
    #[test]
    fn uv_clipping_scale_half_left_edge() {
        // 100×100 node, scale=0.5 → visual 50×50
        // centre at (50, 50) → TL at (25, 25), TR at (75, 25)
        // Clip left edge at 35 → cuts 10 screen-px = 20 local-px from TL
        let node_rect = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100., 100.),
        };
        let clip = Rect {
            min: Vec2::new(35., 0.),
            max: Vec2::new(200., 200.),
        };
        let center = Vec2::new(50., 50.);
        let transform = scale_transform(center, 0.5);
        let atlas_extent = Vec2::new(100., 100.);

        let fixed = compute_clipped_uvs(transform, node_rect, clip, atlas_extent);

        let expected_tl_uv = Vec2::new(0.2, 0.); // 20 local-px / 100 = 0.2
        assert!(
            approx_eq(fixed[0], expected_tl_uv),
            "scale=0.5 TL UV wrong: got {:?}, expected {:?}",
            fixed[0],
            expected_tl_uv
        );
    }

    // -----------------------------------------------------------------------
    // When the node is fully inside the clip rect, all positions_diff are
    // zero and UVs should be the standard corner values regardless of scale.
    #[test]
    fn uv_no_clipping_scale_three() {
        let node_rect = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100., 100.),
        };
        // Very large clip — nothing is clipped.
        let clip = Rect {
            min: Vec2::new(-9999., -9999.),
            max: Vec2::new(9999., 9999.),
        };
        let center = Vec2::new(150., 150.);
        let transform = scale_transform(center, 3.0);
        let atlas_extent = Vec2::new(100., 100.);

        let fixed = compute_clipped_uvs(transform, node_rect, clip, atlas_extent);

        // Standard corner UVs (TL, TR, BR, BL).
        assert!(approx_eq(fixed[0], Vec2::new(0., 0.)), "TL: {:?}", fixed[0]);
        assert!(approx_eq(fixed[1], Vec2::new(1., 0.)), "TR: {:?}", fixed[1]);
        assert!(approx_eq(fixed[2], Vec2::new(1., 1.)), "BR: {:?}", fixed[2]);
        assert!(approx_eq(fixed[3], Vec2::new(0., 1.)), "BL: {:?}", fixed[3]);
    }

    // -----------------------------------------------------------------------
    // `point` attribute used for border-radius SDF: with scale=3 and the
    // right edge clipped by 100 screen-px, the TR `point.x` should be
    // 50 - 100/3 ≈ 16.67 (not 50 - 100 = -50 as the old code gave).
    #[test]
    fn sdf_point_clipping_scale_three() {
        let rect_size = Vec2::new(100., 100.);
        let center = Vec2::new(150., 150.);
        let scale = 3.0_f32;
        let transform = Affine2 {
            matrix2: Mat2::from_diagonal(Vec2::splat(scale)),
            translation: center,
        };

        let positions = QUAD_VERTEX_POSITIONS
            .map(|pos| transform.transform_point2(pos * rect_size));

        let clip = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(200., 200.),
        };

        let positions_diff = [
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
        ];

        let inv = transform.matrix2.inverse();
        let local_positions_diff = positions_diff.map(|d| inv * d);

        let base_points = QUAD_VERTEX_POSITIONS.map(|pos| pos * rect_size);

        // Fixed: use local_positions_diff
        let fixed_points = [
            base_points[0] + local_positions_diff[0],
            base_points[1] + local_positions_diff[1],
            base_points[2] + local_positions_diff[2],
            base_points[3] + local_positions_diff[3],
        ];

        // Old (broken): used screen-space positions_diff directly
        let broken_points = [
            base_points[0] + positions_diff[0],
            base_points[1] + positions_diff[1],
            base_points[2] + positions_diff[2],
            base_points[3] + positions_diff[3],
        ];

        // TR corner: screen clip of -100 px on x → local clip of -100/3 ≈ -33.33
        // base TR point.x = 50.  Fixed: 50 - 33.33 = 16.67.  Broken: 50 - 100 = -50.
        let expected_tr_point_x = 50. - 100. / scale; // ≈ 16.67
        assert!(
            (fixed_points[1].x - expected_tr_point_x).abs() < EPS,
            "fixed TR point.x wrong: got {}, expected {}",
            fixed_points[1].x,
            expected_tr_point_x
        );
        assert!(
            (broken_points[1].x - (-50.)).abs() < EPS,
            "broken TR point.x should be -50 (old bug value), got {}",
            broken_points[1].x
        );
    }
}
