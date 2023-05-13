use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::{Bundle, Component, DetectChanges, ReflectComponent},
    query::With,
    schedule::IntoSystemConfigs,
    system::{Local, Query, Res, ResMut},
    world::{Mut, Ref},
};
use bevy_math::Vec2;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_render::{
    camera::CameraUpdateSystem,
    prelude::Color,
    texture::Image,
    view::{ComputedVisibility, Visibility},
};
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextAlignment, TextError, TextLayoutInfo,
    TextMeasureInfo, TextPipeline, TextSection, TextSettings, TextStyle, YAxisOrientation,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui::{BackgroundColor, ContentSize, Measure, Node, UiScale};
use bevy_ui::{FocusPolicy, Style, UiSystem, ZIndex};
use bevy_window::{PrimaryWindow, Window};
use taffy::style::AvailableSpace;

/// Text system flags
///
/// Used internally by [`measure_text_system`] and [`text_system`] to schedule text for processing.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct TextFlags {
    /// If set a new measure function for the text node will be created
    needs_new_measure_func: bool,
    /// If set the text will be recomputed
    needs_recompute: bool,
}

impl Default for TextFlags {
    fn default() -> Self {
        Self {
            needs_new_measure_func: true,
            needs_recompute: true,
        }
    }
}

#[derive(Clone)]
pub struct TextMeasure {
    pub info: TextMeasureInfo,
}

impl Measure for TextMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        available_width: AvailableSpace,
        _available_height: AvailableSpace,
    ) -> Vec2 {
        let x = width.unwrap_or_else(|| match available_width {
            AvailableSpace::Definite(x) => x.clamp(
                self.info.min_width_content_size.x,
                self.info.max_width_content_size.x,
            ),
            AvailableSpace::MinContent => self.info.min_width_content_size.x,
            AvailableSpace::MaxContent => self.info.max_width_content_size.x,
        });

        height
            .map_or_else(
                || match available_width {
                    AvailableSpace::Definite(_) => self.info.compute_size(Vec2::new(x, f32::MAX)),
                    AvailableSpace::MinContent => Vec2::new(x, self.info.min_width_content_size.y),
                    AvailableSpace::MaxContent => Vec2::new(x, self.info.max_width_content_size.y),
                },
                |y| Vec2::new(x, y),
            )
            .ceil()
    }
}

#[inline]
fn create_text_measure(
    fonts: &Assets<Font>,
    text_pipeline: &mut TextPipeline,
    scale_factor: f64,
    text: Ref<Text>,
    mut content_size: Mut<ContentSize>,
    mut text_flags: Mut<TextFlags>,
) {
    match text_pipeline.create_text_measure(
        fonts,
        &text.sections,
        scale_factor,
        text.alignment,
        text.linebreak_behavior,
    ) {
        Ok(measure) => {
            content_size.set(TextMeasure { info: measure });

            // Text measure func created succesfully, so set `TextFlags` to schedule a recompute
            text_flags.needs_new_measure_func = false;
            text_flags.needs_recompute = true;
        }
        Err(TextError::NoSuchFont) => {
            // Try again next frame
            text_flags.needs_new_measure_func = true;
        }
        Err(e @ TextError::FailedToAddGlyph(_)) => {
            panic!("Fatal error when processing text: {e}.");
        }
    };
}

/// Creates a `Measure` for text nodes that allows the UI to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
pub fn measure_text_system(
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(Ref<Text>, &mut ContentSize, &mut TextFlags), With<Node>>,
) {
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // scale factor unchanged, only create new measure funcs for modified text
        for (text, content_size, text_flags) in text_query.iter_mut() {
            if text.is_changed() || text_flags.needs_new_measure_func {
                create_text_measure(
                    &fonts,
                    &mut text_pipeline,
                    scale_factor,
                    text,
                    content_size,
                    text_flags,
                );
            }
        }
    } else {
        // scale factor changed, create new measure funcs for all text
        *last_scale_factor = scale_factor;

        for (text, content_size, text_flags) in text_query.iter_mut() {
            create_text_measure(
                &fonts,
                &mut text_pipeline,
                scale_factor,
                text,
                content_size,
                text_flags,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
#[inline]
fn queue_text(
    fonts: &Assets<Font>,
    text_pipeline: &mut TextPipeline,
    font_atlas_warning: &mut FontAtlasWarning,
    font_atlas_set_storage: &mut Assets<FontAtlasSet>,
    texture_atlases: &mut Assets<TextureAtlas>,
    textures: &mut Assets<Image>,
    text_settings: &TextSettings,
    scale_factor: f64,
    text: &Text,
    node: Ref<Node>,
    mut text_flags: Mut<TextFlags>,
    mut text_layout_info: Mut<TextLayoutInfo>,
) {
    // Skip the text node if it is waiting for a new measure func
    if !text_flags.needs_new_measure_func {
        let physical_node_size = node.physical_size(scale_factor);

        match text_pipeline.queue_text(
            fonts,
            &text.sections,
            scale_factor,
            text.alignment,
            text.linebreak_behavior,
            physical_node_size,
            font_atlas_set_storage,
            texture_atlases,
            textures,
            text_settings,
            font_atlas_warning,
            YAxisOrientation::TopToBottom,
        ) {
            Err(TextError::NoSuchFont) => {
                // There was an error processing the text layout, try again next frame
                text_flags.needs_recompute = true;
            }
            Err(e @ TextError::FailedToAddGlyph(_)) => {
                panic!("Fatal error when processing text: {e}.");
            }
            Ok(info) => {
                *text_layout_info = info;
                text_flags.needs_recompute = false;
            }
        }
    }
}

/// Updates the layout and size information whenever the text or style is changed.
/// This information is computed by the `TextPipeline` on insertion, then stored.
///
/// ## World Resources
///
/// [`ResMut<Assets<Image>>`](Assets<Image>) -- This system only adds new [`Image`] assets.
/// It does not modify or observe existing ones.
#[allow(clippy::too_many_arguments)]
pub fn text_system(
    mut textures: ResMut<Assets<Image>>,
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    text_settings: Res<TextSettings>,
    mut font_atlas_warning: ResMut<FontAtlasWarning>,
    ui_scale: Res<UiScale>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_query: Query<(Ref<Node>, &Text, &mut TextLayoutInfo, &mut TextFlags)>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    if *last_scale_factor == scale_factor {
        // Scale factor unchanged, only recompute text for modified text nodes
        for (node, text, text_layout_info, text_flags) in text_query.iter_mut() {
            if node.is_changed() || text_flags.needs_recompute {
                queue_text(
                    &fonts,
                    &mut text_pipeline,
                    &mut font_atlas_warning,
                    &mut font_atlas_set_storage,
                    &mut texture_atlases,
                    &mut textures,
                    &text_settings,
                    scale_factor,
                    text,
                    node,
                    text_flags,
                    text_layout_info,
                );
            }
        }
    } else {
        // Scale factor changed, recompute text for all text nodes
        *last_scale_factor = scale_factor;

        for (node, text, text_layout_info, text_flags) in text_query.iter_mut() {
            queue_text(
                &fonts,
                &mut text_pipeline,
                &mut font_atlas_warning,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                &text_settings,
                scale_factor,
                text,
                node,
                text_flags,
                text_layout_info,
            );
        }
    }
}

/// A plugin for adding text rendering
#[derive(Default)]
pub struct TextPlugin;

impl Plugin for TextPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                measure_text_system
                    .before(UiSystem::Layout)
                    // Potential conflict: `Assets<Image>`
                    // In practice, they run independently since `bevy_render::camera_update_system`
                    // will only ever observe its own render target, and `widget::measure_text_system`
                    // will never modify a pre-existing `Image` asset.
                    .ambiguous_with(CameraUpdateSystem)
                    // Potential conflict: `Assets<Image>`
                    // Since both systems will only ever insert new [`Image`] assets,
                    // they will never observe each other's effects.
                    .ambiguous_with(bevy_text::update_text2d_layout),
                text_system.after(UiSystem::Layout),
            ),
        );
    }
}

/// A UI node that is text
#[derive(Bundle, Debug)]
pub struct TextBundle {
    /// Describes the logical size of the node
    pub node: Node,
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// Text layout information
    pub text_layout_info: TextLayoutInfo,
    /// Text system flags
    pub text_flags: TextFlags,
    /// The calculated size based on the given image
    pub calculated_size: ContentSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// The background color that will fill the containing node
    pub background_color: BackgroundColor,
}

impl Default for TextBundle {
    fn default() -> Self {
        Self {
            text: Default::default(),
            text_layout_info: Default::default(),
            text_flags: Default::default(),
            calculated_size: Default::default(),
            // Transparent background
            background_color: BackgroundColor(Color::NONE),
            node: Default::default(),
            style: Default::default(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
            z_index: Default::default(),
        }
    }
}

impl TextBundle {
    /// Create a [`TextBundle`] from a single section.
    ///
    /// See [`Text::from_section`] for usage.
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: Text::from_section(value, style),
            ..Default::default()
        }
    }

    /// Create a [`TextBundle`] from a list of sections.
    ///
    /// See [`Text::from_sections`] for usage.
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            text: Text::from_sections(sections),
            ..Default::default()
        }
    }

    /// Returns this [`TextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub const fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text.alignment = alignment;
        self
    }

    /// Returns this [`TextBundle`] with a new [`Style`].
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns this [`TextBundle`] with a new [`BackgroundColor`].
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = BackgroundColor(color);
        self
    }
}
