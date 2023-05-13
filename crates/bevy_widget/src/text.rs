use bevy_app::{App, Plugin, PostUpdate};
use bevy_ui::{ContentSize, Measure, Node, UiScale, BackgroundColor};
use bevy_asset::Assets;
use bevy_ecs::{
    entity::Entity,
    prelude::Bundle,
    query::{Changed, Or, With},
    system::{Local, ParamSet, Query, Res, ResMut}, schedule::IntoSystemConfigs,
};
use bevy_math::Vec2;
use bevy_render::{
    camera::CameraUpdateSystem,
    texture::Image,
    view::{ComputedVisibility, Visibility}, prelude::Color,
};
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{
    Font, FontAtlasSet, FontAtlasWarning, Text, TextAlignment, TextError, TextLayoutInfo, TextMeasureInfo,
    TextPipeline, TextSection, TextSettings, TextStyle, YAxisOrientation,
};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui::{FocusPolicy, Style, UiSystem, ZIndex};
use bevy_window::{PrimaryWindow, Window};
use taffy::style::AvailableSpace;

fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
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

/// Creates a `Measure` for text nodes that allows the UI to determine the appropriate amount of space
/// to provide for the text given the fonts, the text itself and the constraints of the layout.
pub fn measure_text_system(
    mut queued_text: Local<Vec<Entity>>,
    mut last_scale_factor: Local<f64>,
    fonts: Res<Assets<Font>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    mut text_pipeline: ResMut<TextPipeline>,
    mut text_queries: ParamSet<(
        Query<Entity, (Changed<Text>, With<Node>)>,
        Query<Entity, (With<Text>, With<Node>)>,
        Query<(&Text, &mut ContentSize)>,
    )>,
) {
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text has changed to the local queue
        for entity in text_queries.p0().iter() {
            if !queued_text.contains(&entity) {
                queued_text.push(entity);
            }
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    if queued_text.is_empty() {
        return;
    }

    let mut new_queue = Vec::new();
    let mut query = text_queries.p2();
    for entity in queued_text.drain(..) {
        if let Ok((text, mut content_size)) = query.get_mut(entity) {
            match text_pipeline.create_text_measure(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behavior,
            ) {
                Ok(measure) => {
                    content_size.set(TextMeasure { info: measure });
                }
                Err(TextError::NoSuchFont) => {
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
            };
        }
    }
    *queued_text = new_queue;
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
    mut queued_text: Local<Vec<Entity>>,
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
    mut text_queries: ParamSet<(
        Query<Entity, Or<(Changed<Text>, Changed<Node>)>>,
        Query<Entity, (With<Text>, With<Node>)>,
        Query<(&Node, &Text, &mut TextLayoutInfo)>,
    )>,
) {
    // TODO: Support window-independent scaling: https://github.com/bevyengine/bevy/issues/5621
    let window_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.);

    let scale_factor = ui_scale.scale * window_scale_factor;

    #[allow(clippy::float_cmp)]
    if *last_scale_factor == scale_factor {
        // Adds all entities where the text or the style has changed to the local queue
        for entity in text_queries.p0().iter() {
            if !queued_text.contains(&entity) {
                queued_text.push(entity);
            }
        }
    } else {
        // If the scale factor has changed, queue all text
        for entity in text_queries.p1().iter() {
            queued_text.push(entity);
        }
        *last_scale_factor = scale_factor;
    }

    let mut new_queue = Vec::new();
    let mut text_query = text_queries.p2();
    for entity in queued_text.drain(..) {
        if let Ok((node, text, mut text_layout_info)) = text_query.get_mut(entity) {
            let node_size = Vec2::new(
                scale_value(node.size().x, scale_factor),
                scale_value(node.size().y, scale_factor),
            );

            match text_pipeline.queue_text(
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.linebreak_behavior,
                node_size,
                &mut font_atlas_set_storage,
                &mut texture_atlases,
                &mut textures,
                text_settings.as_ref(),
                &mut font_atlas_warning,
                YAxisOrientation::TopToBottom,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    new_queue.push(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {e}.");
                }
                Ok(info) => {
                    *text_layout_info = info;
                }
            }
        }
    }
    *queued_text = new_queue;
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
#[derive(Bundle, Debug)]pub struct TextBundle {
    /// Describes the logical size of the node
    pub node: Node,
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// Text layout information
    pub text_layout_info: TextLayoutInfo,
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
