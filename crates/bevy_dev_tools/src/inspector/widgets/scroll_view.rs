use bevy_app::{App, Plugin};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_ui::prelude::*;
use bevy_ui::{ScrollPosition, RelativeCursorPosition};
use bevy_core_widgets::{CoreScrollbar, CoreScrollbarThumb, ControlOrientation};
use super::core_scroll_area::{CoreScrollArea, ScrollContent};
use core::default::Default;

/// A styled scroll view widget with built-in scrollbars, padding, and visual styling.
/// This is the opinionated, high-level scroll widget that users will typically interact with.
#[derive(Component)]
pub struct ScrollView {
    /// Whether to show scrollbars
    pub show_scrollbars: bool,
    /// Scrollbar width
    pub scrollbar_width: f32,
    /// Corner radius for rounded corners
    pub corner_radius: f32,
    /// Background color
    pub background_color: Color,
    /// Border color
    pub border_color: Color,
    /// Border width
    pub border_width: f32,
    /// Content padding
    pub padding: UiRect,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self {
            show_scrollbars: true,
            scrollbar_width: 12.0,
            corner_radius: 4.0,
            background_color: Color::srgb(0.14, 0.14, 0.14),
            border_color: Color::srgb(0.35, 0.35, 0.35),
            border_width: 1.0,
            padding: UiRect::all(Val::Px(8.0)),
        }
    }
}

/// Plugin for the high-level scroll view widget
pub struct ScrollViewPlugin;

impl Plugin for ScrollViewPlugin {
    fn build(&self, _app: &mut App) {
        // ScrollView now uses Bevy's built-in scrolling and bevy_core_widgets
        // No custom systems needed
    }
}

/// Builder for creating scroll views
pub struct ScrollViewBuilder {
    scroll_view: ScrollView,
    scroll_area: CoreScrollArea,
    scroll_id: u32,
}

impl ScrollViewBuilder {
    pub fn new() -> Self {
        let scroll_id = rand::random();
        Self {
            scroll_view: ScrollView::default(),
            scroll_area: CoreScrollArea {
                scroll_id,
                ..CoreScrollArea::default()
            },
            scroll_id,
        }
    }

    pub fn with_background_color(mut self, color: Color) -> Self {
        self.scroll_view.background_color = color;
        self
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.scroll_view.border_color = color;
        self
    }

    pub fn with_padding(mut self, padding: UiRect) -> Self {
        self.scroll_view.padding = padding;
        self
    }

    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.scroll_view.corner_radius = radius;
        self
    }

    pub fn with_scrollbars(mut self, show: bool) -> Self {
        self.scroll_view.show_scrollbars = show;
        self
    }

    pub fn with_scroll_sensitivity(mut self, sensitivity: f32) -> Self {
        self.scroll_area.scroll_sensitivity = sensitivity;
        self
    }

    pub fn with_scroll_id(mut self, id: u32) -> Self {
        self.scroll_id = id;
        self.scroll_area.scroll_id = id;
        self
    }

    /// Spawn the complete scroll view hierarchy
    pub fn spawn(self, parent: &mut ChildSpawnerCommands) -> Entity {
        let show_scrollbars = self.scroll_view.show_scrollbars;
        let scrollbar_width = self.scroll_view.scrollbar_width;
        let padding = self.scroll_view.padding;
        let border_width = self.scroll_view.border_width;
        let background_color = self.scroll_view.background_color;
        let border_color = self.scroll_view.border_color;
        
        // Create the outer container with relative positioning for absolute scrollbar
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Relative,
                border: UiRect::all(Val::Px(border_width)),
                ..Default::default()
            },
            BackgroundColor(background_color),
            BorderColor::all(border_color),
            self.scroll_view,
        )).with_children(|parent| {
            // The scrollable content area with proper Bevy scrolling setup
            let scroll_area_entity = parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    padding: if show_scrollbars {
                        UiRect {
                            left: padding.left,
                            right: Val::Px(match padding.right {
                                Val::Px(val) => val + scrollbar_width + 4.0,
                                _ => scrollbar_width + 4.0,
                            }),
                            top: padding.top,
                            bottom: padding.bottom,
                        }
                    } else {
                        padding
                    },
                    overflow: Overflow::scroll(), // Key: use scroll instead of clip
                    ..Default::default()
                },
                BackgroundColor(Color::NONE),
                ScrollPosition::default(), // Key: Bevy's scroll position
                RelativeCursorPosition::default(), // Key: for mouse wheel detection
                self.scroll_area,
                ScrollContent {
                    scroll_area_id: self.scroll_id,
                },
            )).id();

            // Vertical scrollbar (if enabled) - absolutely positioned to the right
            if show_scrollbars {
                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: Val::Px(border_width + 2.0), // Account for border and small margin
                        top: Val::Px(border_width),
                        bottom: Val::Px(border_width),
                        width: Val::Px(scrollbar_width),
                        ..Default::default()
                    },
                    BackgroundColor(Color::srgba(0.2, 0.2, 0.2, 0.8)),
                    CoreScrollbar::new(scroll_area_entity, ControlOrientation::Vertical, 20.0),
                )).with_children(|parent| {
                    // Scrollbar thumb using Bevy's CoreScrollbarThumb
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Percent(100.0),
                            ..Default::default()
                        },
                        BackgroundColor(Color::srgba(0.4, 0.4, 0.4, 0.9)),
                        BorderRadius::all(Val::Px(2.0)),
                        CoreScrollbarThumb,
                    ));
                });
            }
        }).id()
    }
}

impl Default for ScrollViewBuilder {
    fn default() -> Self {
        Self::new()
    }
}
