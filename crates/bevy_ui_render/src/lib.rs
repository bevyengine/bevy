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
pub use image::ImageNodeAssetChangedSystems;
mod pipeline;
pub mod render_pass;
mod text;
pub mod ui_material;
mod ui_material_pipeline;
pub mod ui_texture_slice_pipeline;

#[cfg(feature = "bevy_ui_debug")]
mod debug_overlay;
pub mod extract_layout;

use bevy_a11y::AccessibilitySystems;
use bevy_camera::{Camera, Camera2d, Camera3d, RenderTarget};
use bevy_ecs::entity::{EntityHashSet, EntityIndexMap};
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_render::camera::{extract_cameras, CameraMainPassTextureFormats};
use bevy_render::renderer::RenderQueue;
use bevy_render::sync_world::{MainEntityHashMap, MainEntityHashSet};
use bevy_shader::load_shader_library;
use bevy_sprite_render::SpriteAssetEvents;
use bevy_ui::widget::{ImageNode, ImageNodeSize, NodeImageMode, ViewportNode};
use bevy_ui::{
    BackgroundColor, BorderColor, ComputedUiTargetCamera, Node, OuterColor, Outline,
    ResolvedBorderRadius, UiSystems, VisualBox,
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
    renderer::RenderDevice,
    sync_world::{MainEntity, RenderEntity},
    texture::GpuImage,
    view::{ExtractedView, RetainedViewEntity, ViewUniforms},
    Extract, ExtractSchedule, GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_sprite::BorderRect;
#[cfg(feature = "bevy_ui_debug")]
pub use debug_overlay::{UiDebugOutline, UiDebugOverlay};

use gradient::GradientPlugin;

use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_transform::components::GlobalTransform;
use box_shadow::BoxShadowPlugin;
use bytemuck::{Pod, Zeroable};
use core::ops::Range;
use std::mem;

pub use pipeline::*;
pub use render_pass::*;
pub use ui_material_pipeline::*;
use ui_texture_slice_pipeline::UiTextureSlicerPlugin;

use crate::extract_layout::{ExtractedUiLayout, ExtractedUiNodeLayout};
use crate::shader_flags::INVERT;
use crate::text::{extract_text, queue_text, ExtractedGlyphLayout, ExtractedGlyphLayouts};

pub mod prelude {
    #[cfg(feature = "bevy_ui_debug")]
    pub use crate::debug_overlay::{UiDebugOutline, UiDebugOverlay};

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
    ExtractCameraViews,
    ExtractBoxShadows,
    ExtractBackgrounds,
    ExtractTextureSlice,
    ExtractText,
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
        app.init_resource::<UiDebugOverlay>();

        app.add_systems(
            PostUpdate,
            (
                image::mark_images_as_changed_if_their_assets_changed,
                image::update_texture_atlas_layout_components,
            )
                .chain()
                .in_set(ImageNodeAssetChangedSystems)
                .after(UiSystems::Content)
                .after(AssetEventSystems)
                .after(AccessibilitySystems::Update),
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        #[cfg(feature = "bevy_ui_debug")]
        render_app.init_resource::<debug_overlay::ExtractedUiDebugOverlay>();

        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<UiPipeline>>()
            .init_gpu_resource::<ImageNodeBindGroups>()
            .init_gpu_resource::<UiMeta>()
            .init_resource::<ExtractedUiLayout>()
            .init_resource::<ExtractedUiNodeStyles>()
            .allow_ambiguous_resource::<ExtractedUiNodeStyles>()
            .init_resource::<ExtractedGlyphLayouts>()
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
                    RenderUiSystems::ExtractTextureSlice,
                    RenderUiSystems::ExtractText,
                    RenderUiSystems::ExtractDebug,
                )
                    .chain_weak(),
            )
            .add_systems(RenderStartup, init_ui_pipeline)
            .add_systems(
                ExtractSchedule,
                (
                    extract_layout::extract_ui_layout,
                    extract_ui_camera_view
                        .after(extract_cameras)
                        .in_set(RenderUiSystems::ExtractCameraViews),
                    extract_uinode_styles.in_set(RenderUiSystems::ExtractBackgrounds),
                    extract_text.in_set(RenderUiSystems::ExtractText),
                ),
            )
            .add_systems(
                Render,
                (
                    queue_uinodes.in_set(RenderSystems::Queue),
                    queue_text.in_set(RenderSystems::Queue),
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

        #[cfg(feature = "bevy_ui_debug")]
        render_app
            .add_systems(
                ExtractSchedule,
                debug_overlay::extract_debug_overlay
                    .after(extract_layout::extract_ui_layout)
                    .in_set(RenderUiSystems::ExtractDebug),
            )
            .add_systems(
                Render,
                debug_overlay::queue_debug_overlay.in_set(RenderSystems::Queue),
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
            camera_entity: None,
            render_entity: None,
        }
    }
}

/// Helper for mapping UI target camera entities to their corresponding render entities,
/// with caching to avoid repeated lookups for the same camera.
pub struct UiCameraMapper<'w, 's> {
    mapping: &'w Query<'w, 's, RenderEntity>,
    /// Cached camera entity from the last successful `map` call.
    camera_entity: Option<Entity>,
    /// Cached camera entity from the last successful `map` call.
    render_entity: Option<Entity>,
}

impl<'w, 's> UiCameraMapper<'w, 's> {
    /// Returns the render entity corresponding to the given [`ComputedUiTargetCamera`]'s camera, or none if no corresponding entity was found.
    pub fn map(&mut self, computed_target: &ComputedUiTargetCamera) -> Option<Entity> {
        let camera_entity = computed_target.get()?;
        if self.camera_entity != Some(camera_entity) {
            let new_render_camera_entity = self.mapping.get(camera_entity).ok()?;
            self.render_entity = Some(new_render_camera_entity);
            self.camera_entity = Some(camera_entity);
        }

        self.render_entity
    }

    /// Returns the cached camera entity from the last successful `map` call.
    pub fn current_camera(&self) -> Option<Entity> {
        self.camera_entity
    }
}

#[derive(Default)]
pub struct ExtractedUiNodeStyle {
    pub background_color: LinearRgba,
    pub border_color: [LinearRgba; 4],
    pub outer_color: LinearRgba,
    pub image_color: LinearRgba,
    pub outline_color: LinearRgba,
    pub image: Option<AssetId<Image>>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub image_rect: Option<Rect>,
    pub image_size: Vec2,
    pub auto_sized: bool,
    pub visual_box: VisualBox,
    pub use_node_border: bool,
}

impl ExtractedUiNodeStyle {
    pub fn is_visible(&self) -> bool {
        !self.background_color.is_fully_transparent()
            || !self.outer_color.is_fully_transparent()
            || self
                .border_color
                .iter()
                .any(|color| !color.is_fully_transparent())
            || !self.outline_color.is_fully_transparent()
            || self.image.is_some()
    }
}

/// The type of UI node.
/// This is used to determine how to render the UI node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NodeType {
    Rect,
    Inverted,
    Border(u32), // shader flags
}

/// The list of UI nodes, as well as the set of nodes that changed.
///
#[derive(Resource, Default)]
pub struct ExtractedUiNodeStyles {
    pub uinodes: MainEntityHashMap<(Entity, ExtractedUiNodeStyle)>,
    /// UI nodes that changed this frame.
    pub changed_this_frame: MainEntityHashSet,
}

impl ExtractedUiNodeStyles {
    pub fn get_style(
        &self,
        main_entity: MainEntity,
        render_entity: Entity,
    ) -> Option<&ExtractedUiNodeStyle> {
        self.uinodes
            .get(&main_entity)
            .and_then(|(style_render_entity, style)| {
                (render_entity == *style_render_entity).then_some(style)
            })
    }
}

pub fn extract_uinode_styles(
    mut commands: Commands,
    mut extracted_uinodes: ResMut<ExtractedUiNodeStyles>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    camera_query: Extract<Query<(&Camera, &RenderTarget)>>,
    changed_uinode_query: Extract<
        Query<
            (
                Entity,
                &BackgroundColor,
                Option<&OuterColor>,
                Option<&ImageNode>,
                Option<&ImageNodeSize>,
                Option<&BorderColor>,
                Option<&Outline>,
                Option<&ViewportNode>,
            ),
            Or<(
                Changed<BackgroundColor>,
                Changed<OuterColor>,
                Changed<ImageNode>,
                Changed<ImageNodeSize>,
                Changed<BorderColor>,
                Changed<Outline>,
                Changed<ViewportNode>,
            )>,
        >,
    >,
    uinode_query: Extract<
        Query<(
            Entity,
            &BackgroundColor,
            Option<&OuterColor>,
            Option<&ImageNode>,
            Option<&ImageNodeSize>,
            Option<&BorderColor>,
            Option<&Outline>,
            Option<&ViewportNode>,
        )>,
    >,
    (
        mut removed_background_color_query,
        mut removed_outer_color_query,
        mut removed_image_node_query,
        mut removed_image_node_size_query,
        mut removed_border_color_query,
        mut removed_outline_query,
        mut removed_viewport_node_query,
    ): (
        Extract<RemovedComponents<BackgroundColor>>,
        Extract<RemovedComponents<OuterColor>>,
        Extract<RemovedComponents<ImageNode>>,
        Extract<RemovedComponents<ImageNodeSize>>,
        Extract<RemovedComponents<BorderColor>>,
        Extract<RemovedComponents<Outline>>,
        Extract<RemovedComponents<ViewportNode>>,
    ),
    mut removed_uinodes: Local<EntityHashSet>,
) {
    extracted_uinodes.changed_this_frame.clear();
    removed_uinodes.clear();
    removed_uinodes.extend(
        removed_background_color_query
            .read()
            .chain(removed_outer_color_query.read())
            .chain(removed_image_node_query.read())
            .chain(removed_image_node_size_query.read())
            .chain(removed_border_color_query.read())
            .chain(removed_outline_query.read())
            .chain(removed_viewport_node_query.read()),
    );

    for (
        entity,
        background_color,
        maybe_outer_color,
        maybe_image,
        maybe_image_size,
        maybe_border_color,
        maybe_outline,
        maybe_viewport_node,
    ) in changed_uinode_query
        .iter()
        .chain(uinode_query.iter_many(removed_uinodes.iter()))
    {
        let main_entity = entity.into();

        // Already updated this entity
        if !extracted_uinodes.changed_this_frame.insert(main_entity) {
            continue;
        }

        let mut style = ExtractedUiNodeStyle::default();

        if let Some(image) = maybe_image
            && !image.color.is_fully_transparent()
            && image.image.id() != TRANSPARENT_IMAGE_HANDLE.id()
            && !image.image_mode.uses_slices()
        {
            let atlas_rect = image
                .texture_atlas
                .as_ref()
                .and_then(|s| s.texture_rect(&texture_atlases))
                .map(|r| r.as_rect());

            style.image_rect = match (atlas_rect, image.rect) {
                (None, None) => None,
                (None, Some(image_rect)) => Some(image_rect),
                (Some(atlas_rect), None) => Some(atlas_rect),
                (Some(atlas_rect), Some(mut image_rect)) => {
                    image_rect.min += atlas_rect.min;
                    image_rect.max += atlas_rect.min;
                    Some(image_rect)
                }
            };
            style.image_color = image.color.into();
            style.image = Some(image.image.id());
            style.flip_x = image.flip_x;
            style.flip_y = image.flip_y;
            style.image_size = maybe_image_size
                .map(|image_size| image_size.size().as_vec2())
                .unwrap_or_default();
            style.auto_sized = matches!(image.image_mode, NodeImageMode::Auto);
            style.visual_box = image.visual_box;
        }

        if let Some(viewport_node) = maybe_viewport_node
            && let Some(camera_entity) = viewport_node.camera
            && let Some(image) = camera_query
                .get(camera_entity)
                .ok()
                .and_then(|(_, render_target)| render_target.as_image())
        {
            style.image_color = LinearRgba::WHITE;
            style.image = Some(image.id());
            style.flip_x = false;
            style.flip_y = false;
            style.image_rect = None;
            style.image_size = Vec2::ZERO;
            style.auto_sized = false;
            style.visual_box = VisualBox::BorderBox;
            style.use_node_border = true;
        }

        style.background_color = background_color.to_linear();
        style.border_color = maybe_border_color
            .map(|&border_color| border_color.into())
            .unwrap_or([LinearRgba::NONE; 4]);
        style.outer_color = maybe_outer_color
            .map(|outer_color| outer_color.0.to_linear())
            .unwrap_or(LinearRgba::NONE);
        style.outline_color = maybe_outline
            .map(|outline| outline.color.to_linear())
            .unwrap_or(LinearRgba::NONE);

        if style.is_visible() {
            match extracted_uinodes.uinodes.entry(main_entity) {
                Entry::Occupied(mut entry) => entry.get_mut().1 = style,
                Entry::Vacant(entry) => {
                    entry.insert((commands.spawn_empty().id(), style));
                }
            }
        } else if let Some((render_entity, _)) = extracted_uinodes.uinodes.remove(&main_entity) {
            commands.entity(render_entity).despawn();
        }
    }

    for entity in removed_uinodes.drain() {
        let main_entity = MainEntity::from(entity);
        if uinode_query.contains(main_entity.entity()) {
            continue;
        }
        extracted_uinodes.changed_this_frame.insert(main_entity);
        let Some((render_entity, _)) = extracted_uinodes.uinodes.remove(&main_entity) else {
            continue;
        };
        commands.entity(render_entity).despawn();
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
pub struct UiExtractedView;

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
                Option<&UiAntiAlias>,
                Option<&BoxShadowSamples>,
            ),
            Or<(With<Camera2d>, With<Camera3d>)>,
        >,
    >,
    main_pass_formats: Res<CameraMainPassTextureFormats>,
    mut live_entities: Local<HashSet<RetainedViewEntity>>,
    mut cached_ui_view_data: Local<MainEntityHashMap<CachedUiViewData>>,
    mut removed_cameras_query: Extract<RemovedComponents<Camera>>,
    mut cameras_updated_this_frame: Local<MainEntityHashSet>,
) {
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub(crate) struct UiVertex {
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

// Indexed buffers for UI vertices
#[derive(Resource)]
pub struct UiMeta {
    pub(crate) vertices: RawBufferVec<UiVertex>,
    pub(crate) indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
    texture_changes: Vec<(Range<u32>, AssetId<Image>)>,
}

impl Default for UiMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
            texture_changes: Vec::default(),
        }
    }
}

pub(crate) const QUAD_VERTEX_POSITIONS: [Vec2; 4] = [
    Vec2::new(-0.5, -0.5),
    Vec2::new(0.5, -0.5),
    Vec2::new(0.5, 0.5),
    Vec2::new(-0.5, 0.5),
];

pub(crate) const QUAD_UVS: [Vec2; 4] = [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y];

pub(crate) const QUAD_INDICES: [usize; 6] = [0, 2, 3, 0, 1, 2];

#[derive(Component, Debug)]
pub struct UiBatch {
    pub range: Range<u32>,
    pub texture_changes: Range<u32>,
}

impl UiBatch {
    fn empty(ui_meta: &UiMeta) -> Self {
        UiBatch {
            range: ui_meta.indices.len() as u32..ui_meta.indices.len() as u32,
            texture_changes: ui_meta.texture_changes.len() as u32
                ..ui_meta.texture_changes.len() as u32,
        }
    }
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
    extracted_uinodes: Res<ExtractedUiNodeStyles>,
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

    for (main_entity, (render_entity, _)) in extracted_uinodes.uinodes.iter() {
        let Some(layout) = extracted_layout.layout.get(main_entity) else {
            continue;
        };

        if !layout.visible {
            continue;
        }

        let extracted_camera_entity = layout.extracted_camera;

        if current_camera_entity != extracted_camera_entity {
            current_phase = render_views.get(extracted_camera_entity).ok().and_then(
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
            current_camera_entity = extracted_camera_entity;
        }

        let Some((pipeline, transparent_phase)) = current_phase.as_mut() else {
            continue;
        };

        transparent_phase.add_transient(TransparentUi {
            draw_function,
            pipeline: *pipeline,
            entity: (*render_entity, *main_entity),
            sort_key: FloatOrd(layout.stack_index as f32),
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

// new prepare function design
// draws ui nodes, text layouts and debug outlines
fn prepare_uinodes(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline_cache: Res<PipelineCache>,
    mut ui_meta: ResMut<UiMeta>,
    extracted_styles: Res<ExtractedUiNodeStyles>,
    extracted_glyph_layouts: Res<ExtractedGlyphLayouts>,
    extracted_layout: Res<ExtractedUiLayout>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiPipeline>,
    mut image_bind_groups: ResMut<ImageNodeBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
    #[cfg(feature = "bevy_ui_debug")] extracted_ui_debug_overlay: Res<
        debug_overlay::ExtractedUiDebugOverlay,
    >,
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

    let Some(view_binding) = view_uniforms.uniforms.binding() else {
        // If there are no view bindings, give up.
        return;
    };

    let mut batches: Vec<(Entity, UiBatch)> = Vec::with_capacity(*previous_len);

    // UiMeta contains the vertex and index buffer data and the current view
    ui_meta.vertices.clear();
    ui_meta.indices.clear();
    ui_meta.texture_changes.clear();
    ui_meta.view_bind_group = Some(render_device.create_bind_group(
        "ui_view_bind_group",
        &pipeline_cache.get_bind_group_layout(&ui_pipeline.view_layout),
        &BindGroupEntries::single(view_binding),
    ));

    // for each sorted render phase corresponding to a UI view
    for ui_phase in phases.values_mut() {
        // used to store the index of the first phase item in a batch
        let mut batch_start_item_index = 0;

        let mut batch_open = false;

        for phase_item_index in 0..ui_phase.items.len() {
            let main_entity = ui_phase.items[phase_item_index].main_entity();
            let entity = ui_phase.items[phase_item_index].entity();

            if let Some(style) = extracted_styles.get_style(main_entity, entity) {
                let Some(layout) = extracted_layout.layout.get(&main_entity) else {
                    batch_open = false;
                    continue;
                };
                let image = style
                    .image
                    .filter(|image| gpu_images.get(*image).is_some())
                    .unwrap_or_default();
                let Some(gpu_image) = gpu_images.get(image) else {
                    batch_open = false;
                    continue;
                };

                ui_phase.items[phase_item_index].batch_range =
                    phase_item_index as u32..phase_item_index as u32;

                if !batch_open {
                    batch_start_item_index = phase_item_index;
                    batch_open = true;
                    batches.push((entity, UiBatch::empty(&ui_meta)));
                }
                image_bind_groups.values.entry(image).or_insert_with(|| {
                    render_device.create_bind_group(
                        "ui_material_bind_group",
                        &pipeline_cache.get_bind_group_layout(&ui_pipeline.image_layout),
                        &BindGroupEntries::sequential((
                            &gpu_image.texture_view,
                            &gpu_image.sampler,
                        )),
                    )
                });
                let range_start = ui_meta.indices.len() as u32;
                push_uinode_vertices(&mut ui_meta, layout, style, image, gpu_image);
                let range_end = ui_meta.indices.len() as u32;
                let existing_batch = &mut batches.last_mut().unwrap().1;
                existing_batch.range.end = range_end;
                if existing_batch.texture_changes.is_empty() {
                    if range_start < range_end {
                        ui_meta
                            .texture_changes
                            .push((range_start..range_end, image));
                    }
                } else if let Some((range, texture)) = ui_meta.texture_changes.last_mut() {
                    if image == AssetId::default() || *texture == image {
                        range.end = range_end;
                    } else if range_start < range_end {
                        ui_meta
                            .texture_changes
                            .push((range_start..range_end, image));
                    }
                }
                existing_batch.texture_changes.end = ui_meta.texture_changes.len() as u32;
                ui_phase.items[batch_start_item_index].batch_range_mut().end += 1;
                continue;
            }

            if let Some(style) = extracted_glyph_layouts
                .uinodes
                .get(&main_entity)
                .and_then(|(render_entity, style)| (*render_entity == entity).then_some(style))
            {
                let Some(layout) = extracted_layout.layout.get(&main_entity) else {
                    batch_open = false;
                    continue;
                };
                let image = style
                    .sections
                    .iter()
                    .find_map(|section| {
                        gpu_images
                            .get(section.atlas_texture)
                            .map(|_| section.atlas_texture)
                    })
                    .unwrap_or_default();
                let Some(gpu_image) = gpu_images.get(image) else {
                    batch_open = false;
                    continue;
                };

                ui_phase.items[phase_item_index].batch_range =
                    phase_item_index as u32..phase_item_index as u32;

                if !batch_open {
                    batch_start_item_index = phase_item_index;
                    batch_open = true;
                    batches.push((entity, UiBatch::empty(&ui_meta)));
                }
                image_bind_groups.values.entry(image).or_insert_with(|| {
                    render_device.create_bind_group(
                        "ui_material_bind_group",
                        &pipeline_cache.get_bind_group_layout(&ui_pipeline.image_layout),
                        &BindGroupEntries::sequential((
                            &gpu_image.texture_view,
                            &gpu_image.sampler,
                        )),
                    )
                });

                let existing_batch = &mut batches.last_mut().unwrap().1;
                ui_meta
                    .texture_changes
                    .reserve(style.sections.len().saturating_mul(2).saturating_add(1));
                push_text_vertices(
                    &mut ui_meta,
                    layout,
                    style,
                    &gpu_images,
                    image,
                    existing_batch.texture_changes.start as usize,
                );
                existing_batch.range.end = ui_meta.indices.len() as u32;
                existing_batch.texture_changes.end = ui_meta.texture_changes.len() as u32;
                for (_, texture) in &ui_meta.texture_changes[existing_batch.texture_changes.start
                    as usize
                    ..existing_batch.texture_changes.end as usize]
                {
                    let gpu_image = gpu_images
                        .get(*texture)
                        .expect("Image was checked during quad generation and should still exist");
                    image_bind_groups.values.entry(*texture).or_insert_with(|| {
                        render_device.create_bind_group(
                            "ui_material_bind_group",
                            &pipeline_cache.get_bind_group_layout(&ui_pipeline.image_layout),
                            &BindGroupEntries::sequential((
                                &gpu_image.texture_view,
                                &gpu_image.sampler,
                            )),
                        )
                    });
                }
                ui_phase.items[batch_start_item_index].batch_range_mut().end += 1;
                continue;
            }

            #[cfg(feature = "bevy_ui_debug")]
            {
                if !batch_open {
                    batch_start_item_index = phase_item_index;
                    batches.push((
                        ui_phase.items[batch_start_item_index].entity(),
                        UiBatch::empty(&ui_meta),
                    ));
                }

                let range_start = ui_meta.indices.len() as u32;
                debug_overlay::push_debug_overlay_vertices(
                    &mut ui_meta,
                    &extracted_layout,
                    &extracted_ui_debug_overlay,
                );

                let existing_batch = &mut batches.last_mut().unwrap().1;
                let range_end = ui_meta.indices.len() as u32;
                existing_batch.range.end = range_end;
                if range_start < range_end {
                    if existing_batch.texture_changes.is_empty() {
                        let image = ui_meta
                            .texture_changes
                            .last()
                            .map_or(AssetId::default(), |(_, image)| *image);
                        ui_meta
                            .texture_changes
                            .push((range_start..range_end, image));
                    } else if let Some((range, _)) = ui_meta.texture_changes.last_mut() {
                        range.end = range_end;
                    }
                }
                existing_batch.texture_changes.end = ui_meta.texture_changes.len() as u32;
                ui_phase.items[batch_start_item_index].batch_range_mut().end += 1;
            }

            batch_open = false;
        }
    }

    *previous_len = batches.len();
    commands.try_insert_batch(batches);

    ui_meta.vertices.write_buffer(&render_device, &render_queue);
    ui_meta.indices.write_buffer(&render_device, &render_queue);
}

/// generates vertices from extracted uinode syle and layout
fn push_uinode_vertices(
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
            _ => {}
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

    if !style.outline_color.is_fully_transparent() && 0. < uinode.outline_width() {
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

/// generates vertices from extracted uinode syle and glyph layout
fn push_text_vertices(
    ui_meta: &mut UiMeta,
    layout: &ExtractedUiNodeLayout,
    style: &ExtractedGlyphLayout,
    gpu_images: &RenderAssets<GpuImage>,
    image: AssetId<Image>,
    texture_changes_start: usize,
) {
    let uinode = &layout.uinode;
    let shadow_offset = style.shadow_offset / uinode.inverse_scale_factor();
    let transform = layout.transform
        * Affine2::from_translation(uinode.content_box().min - style.viewport_offset);
    let clip = if style.clip_to_content_box {
        let content_box = uinode.content_box();
        let text_clip = Rect::from_center_size(
            layout.transform.translation + content_box.center(),
            content_box.size(),
        );
        Some(
            layout
                .clip
                .map_or(text_clip, |clip| clip.intersect(text_clip)),
        )
    } else {
        layout.clip
    };
    let (mut texture_range_start, mut current_texture) =
        if texture_changes_start < ui_meta.texture_changes.len() {
            ui_meta
                .texture_changes
                .pop()
                .map(|(range, texture)| (range.start, texture))
                .unwrap()
        } else {
            (ui_meta.indices.len() as u32, image)
        };
    {
        let mut push_quad = |rect: Rect,
                             color: LinearRgba,
                             border_radius: ResolvedBorderRadius,
                             glyph: Option<(Rect, AssetId<Image>)>| {
            let rect_size = rect.size();
            let rect_transform = transform * Affine2::from_translation(rect.center());
            let positions = QUAD_VERTEX_POSITIONS
                .map(|pos| rect_transform.transform_point2(pos * rect_size).extend(0.));
            let positions_diff = if let Some(clip) = clip {
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
            let transformed_rect_size = rect_transform.transform_vector2(rect_size).abs();
            if rect_transform.x_axis[1] == 0.0
                && (positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                    || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y)
            {
                return;
            }

            let (uvs, flags) = if let Some((glyph_rect, atlas_texture)) = glyph {
                let Some(gpu_image) = gpu_images.get(atlas_texture) else {
                    return;
                };
                let range_end = ui_meta.indices.len() as u32;
                if current_texture != atlas_texture {
                    if texture_range_start < range_end {
                        ui_meta
                            .texture_changes
                            .push((texture_range_start..range_end, current_texture));
                    }
                    texture_range_start = range_end;
                    current_texture = atlas_texture;
                }
                let atlas_extent = gpu_image.size_2d().as_vec2();
                (
                    [
                        Vec2::new(
                            glyph_rect.min.x + positions_diff[0].x,
                            glyph_rect.min.y + positions_diff[0].y,
                        ),
                        Vec2::new(
                            glyph_rect.max.x + positions_diff[1].x,
                            glyph_rect.min.y + positions_diff[1].y,
                        ),
                        Vec2::new(
                            glyph_rect.max.x + positions_diff[2].x,
                            glyph_rect.max.y + positions_diff[2].y,
                        ),
                        Vec2::new(
                            glyph_rect.min.x + positions_diff[3].x,
                            glyph_rect.max.y + positions_diff[3].y,
                        ),
                    ]
                    .map(|pos| pos / atlas_extent),
                    shader_flags::TEXTURED,
                )
            } else {
                (
                    [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
                    shader_flags::UNTEXTURED,
                )
            };

            let vertex_start = ui_meta.vertices.len() as u32;
            let color = color.to_f32_array();
            for i in 0..4 {
                ui_meta.vertices.push(UiVertex {
                    position: positions_clipped[i].into(),
                    uv: uvs[i].into(),
                    color,
                    flags: flags | shader_flags::CORNERS[i],
                    radius: border_radius.into(),
                    border: [0.; 4],
                    size: rect_size.into(),
                    point: (QUAD_VERTEX_POSITIONS[i] * rect_size + positions_diff[i]).into(),
                });
            }
            for &index in &QUAD_INDICES {
                ui_meta.indices.push(vertex_start + index as u32);
            }
        };

        let selection_color = if style.focused {
            style.cursor_style.selection_color
        } else {
            style.cursor_style.unfocused_selection_color
        }
        .to_linear();
        for (rect, radius) in &style.selections {
            push_quad(*rect, selection_color, *radius, None);
        }
        for (rect, color) in &style.backgrounds {
            push_quad(*rect, *color, ResolvedBorderRadius::ZERO, None);
        }

        if !style.shadow_color.is_fully_transparent() {
            for section in &style.sections {
                for glyph in &style.glyphs[section.range.start as usize..section.range.end as usize]
                {
                    push_quad(
                        Rect::from_center_size(
                            glyph.translation + shadow_offset,
                            glyph.rect.size(),
                        ),
                        style.shadow_color,
                        ResolvedBorderRadius::ZERO,
                        Some((glyph.rect, section.atlas_texture)),
                    );
                }
            }
            for (rect, _) in style.strike_through.iter().chain(style.underline.iter()) {
                push_quad(
                    Rect::from_center_size(rect.center() + shadow_offset, rect.size()),
                    style.shadow_color,
                    ResolvedBorderRadius::ZERO,
                    None,
                );
            }
        }

        for section in &style.sections {
            for glyph in &style.glyphs[section.range.start as usize..section.range.end as usize] {
                push_quad(
                    Rect::from_center_size(glyph.translation, glyph.rect.size()),
                    glyph.color,
                    ResolvedBorderRadius::ZERO,
                    Some((glyph.rect, section.atlas_texture)),
                );
            }
        }
        for (rect, color) in style.strike_through.iter().chain(style.underline.iter()) {
            push_quad(*rect, *color, ResolvedBorderRadius::ZERO, None);
        }
        if let Some(cursor) = style.cursor {
            push_quad(
                cursor,
                style.cursor_style.color.to_linear(),
                ResolvedBorderRadius::ZERO,
                None,
            );
        }
    }
    let range_end = ui_meta.indices.len() as u32;
    if texture_range_start < range_end {
        ui_meta
            .texture_changes
            .push((texture_range_start..range_end, current_texture));
    }
}
