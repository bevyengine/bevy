use bevy_color::LinearRgba;
use bevy_ecs::prelude::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::prelude::ReflectResource;
use bevy_ecs::resource::Resource;
use bevy_reflect::Reflect;

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
