use bevy::prelude::*;
use bevy::ui::{Style, UiRect, Val, FlexDirection, AlignItems, JustifyContent};

/// Theme-aware colors and styling for widgets
/// This is designed to be compatible with bevy_feathers theming system
#[derive(Resource, Clone)]
pub struct EditorTheme {
    pub background_primary: Color,
    pub background_secondary: Color,
    pub background_tertiary: Color,
    pub border_color: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub accent_color: Color,
    pub selection_color: Color,
    pub hover_color: Color,
    pub active_color: Color,
    pub panel_header_color: Color,
    pub panel_content_color: Color,
}

impl Default for EditorTheme {
    fn default() -> Self {
        Self {
            background_primary: Color::srgb(0.08, 0.08, 0.08),
            background_secondary: Color::srgb(0.12, 0.12, 0.12),
            background_tertiary: Color::srgb(0.15, 0.15, 0.15),
            border_color: Color::srgb(0.3, 0.3, 0.3),
            text_primary: Color::WHITE,
            text_secondary: Color::srgb(0.8, 0.8, 0.8),
            accent_color: Color::srgb(0.2, 0.5, 0.8),
            selection_color: Color::srgb(0.3, 0.4, 0.6),
            hover_color: Color::srgb(0.2, 0.2, 0.2),
            active_color: Color::srgb(0.25, 0.25, 0.25),
            panel_header_color: Color::srgb(0.15, 0.15, 0.15),
            panel_content_color: Color::srgb(0.1, 0.1, 0.1),
        }
    }
}

/// Component that marks entities as theme-aware
#[derive(Component)]
pub struct Themed {
    pub element_type: ThemeElement,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeElement {
    Panel,
    PanelHeader,
    PanelContent,
    Button,
    ButtonHover,
    ButtonActive,
    Text,
    TextSecondary,
    Border,
    Selection,
    Background,
    Accent,
}

/// Bundle for theme-aware UI elements
#[derive(Bundle)]
pub struct ThemedBundle {
    pub themed: Themed,
    pub background_color: BackgroundColor,
    pub border_color: BorderColor,
}

impl ThemedBundle {
    pub fn new(element_type: ThemeElement, theme: &EditorTheme) -> Self {
        let bg_color = match element_type {
            ThemeElement::Panel => theme.background_secondary,
            ThemeElement::PanelHeader => theme.panel_header_color,
            ThemeElement::PanelContent => theme.panel_content_color,
            ThemeElement::Button => theme.background_tertiary,
            ThemeElement::ButtonHover => theme.hover_color,
            ThemeElement::ButtonActive => theme.active_color,
            ThemeElement::Selection => theme.selection_color,
            ThemeElement::Background => theme.background_primary,
            ThemeElement::Accent => theme.accent_color,
            _ => Color::NONE,
        };

        Self {
            themed: Themed { element_type },
            background_color: BackgroundColor(bg_color),
            border_color: BorderColor::all(theme.border_color),
        }
    }
}

/// Plugin for theme management
pub struct ThemePlugin;

impl Plugin for ThemePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorTheme>()
            .add_systems(Update, update_themed_elements);
    }
}

/// System to update themed elements when the theme changes
fn update_themed_elements(
    theme: Res<EditorTheme>,
    mut themed_query: Query<(&Themed, &mut BackgroundColor, &mut BorderColor), Changed<Themed>>,
) {
    if !theme.is_changed() {
        return;
    }

    for (themed, mut bg_color, mut border_color) in &mut themed_query {
        let new_bg_color = match themed.element_type {
            ThemeElement::Panel => theme.background_secondary,
            ThemeElement::PanelHeader => theme.panel_header_color,
            ThemeElement::PanelContent => theme.panel_content_color,
            ThemeElement::Button => theme.background_tertiary,
            ThemeElement::ButtonHover => theme.hover_color,
            ThemeElement::ButtonActive => theme.active_color,
            ThemeElement::Selection => theme.selection_color,
            ThemeElement::Background => theme.background_primary,
            ThemeElement::Accent => theme.accent_color,
            _ => bg_color.0, // Keep current color
        };

        *bg_color = BackgroundColor(new_bg_color);
        *border_color = BorderColor::all(theme.border_color);
    }
}

/// Helper functions for creating themed UI elements

/// Creates a themed text element
pub fn themed_text(
    text: impl Into<String>,
    element_type: ThemeElement,
    theme: &EditorTheme,
    font_size: f32,
) -> impl Bundle {
    let text_color = match element_type {
        ThemeElement::Text => theme.text_primary,
        ThemeElement::TextSecondary => theme.text_secondary,
        _ => theme.text_primary,
    };

    (
        Text::new(text.into()),
        TextColor(text_color),
        TextFont {
            font_size,
            ..default()
        },
        Themed { element_type },
    )
}

/// Creates a themed button
pub fn themed_button(
    theme: &EditorTheme,
    size: (f32, f32),
) -> impl Bundle {
    (
        Button,
        Node {
            width: Val::Px(size.0),
            height: Val::Px(size.1),
            border: UiRect::all(Val::Px(1.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ThemedBundle::new(ThemeElement::Button, theme),
    )
}

/// Creates a themed panel container
pub fn themed_panel(
    theme: &EditorTheme,
    title: impl Into<String>,
) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        ThemedBundle::new(ThemeElement::Panel, theme),
    )
}

/// Extension trait for Commands to easily spawn themed elements
pub trait ThemedCommands {
    fn spawn_themed_text(
        &mut self,
        text: impl Into<String>,
        element_type: ThemeElement,
        font_size: f32,
    ) -> EntityCommands;
    
    fn spawn_themed_button(&mut self, size: (f32, f32)) -> EntityCommands;
    
    fn spawn_themed_panel(&mut self, title: impl Into<String>) -> EntityCommands;
}

impl ThemedCommands for Commands<'_, '_> {
    fn spawn_themed_text(
        &mut self,
        text: impl Into<String>,
        element_type: ThemeElement,
        font_size: f32,
    ) -> EntityCommands {
        let theme = EditorTheme::default(); // In practice, get from resource
        self.spawn(themed_text(text, element_type, &theme, font_size))
    }
    
    fn spawn_themed_button(&mut self, size: (f32, f32)) -> EntityCommands {
        let theme = EditorTheme::default(); // In practice, get from resource
        self.spawn(themed_button(&theme, size))
    }
    
    fn spawn_themed_panel(&mut self, title: impl Into<String>) -> EntityCommands {
        let theme = EditorTheme::default(); // In practice, get from resource
        self.spawn(themed_panel(&theme, title))
    }
}

/// Utility functions for bevy_feathers integration

/// Converts EditorTheme to a format compatible with bevy_feathers UiTheme
/// This would be used when extracting to bevy_feathers
pub fn editor_theme_to_feathers_palette(theme: &EditorTheme) -> FeathersPalette {
    FeathersPalette {
        background: theme.background_primary,
        surface: theme.background_secondary,
        surface_variant: theme.background_tertiary,
        on_surface: theme.text_primary,
        on_surface_variant: theme.text_secondary,
        primary: theme.accent_color,
        outline: theme.border_color,
        outline_variant: theme.border_color.with_alpha(0.5),
    }
}

/// Placeholder for bevy_feathers palette structure
/// This would match the actual bevy_feathers palette when integrating
#[derive(Clone)]
pub struct FeathersPalette {
    pub background: Color,
    pub surface: Color,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub outline: Color,
    pub outline_variant: Color,
}
