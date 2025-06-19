use bevy_app::Propagate;
use bevy_color::{palettes, Color};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    lifecycle::Insert,
    observer::On,
    query::Changed,
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_text::TextColor;
use bevy_ui::{BackgroundColor, BorderColor};

/// A collection of properties that make up a theme.
#[derive(Default, Clone)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<String, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color<'a>(&self, token: &'a str) -> Color {
        let color = self.0.color.get(token);
        match color {
            Some(c) => *c,
            None => {
                warn_once!("Theme color {} not found.", token);
                // Return a bright obnoxious color to make the error obvious.
                palettes::basic::FUCHSIA.into()
            }
        }
    }

    /// Associate a design token with a given color.
    pub fn set_color(&mut self, token: impl Into<String>, color: Color) {
        self.0.color.insert(token.into(), color);
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone, Copy)]
#[require(BackgroundColor)]
#[component(immutable)]
pub struct ThemeBackgroundColor(pub &'static str);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Copy)]
#[require(BorderColor)]
#[component(immutable)]
pub struct ThemeBorderColor(pub &'static str);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone, Copy)]
#[component(immutable)]
pub struct ThemeFontColor(pub &'static str);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component)]
pub struct UseTheme;

/// UX design tokens
pub mod tokens {
    /// Window background
    pub const WINDOW_BG: &str = "window.bg";

    /// Focus ring
    pub const FOCUS_RING: &str = "focus";

    /// Regular text
    pub const TEXT_MAIN: &str = "text.main";
    /// Dim text
    pub const TEXT_DIM: &str = "text.dim";

    // Normal buttons

    /// Regular button background
    pub const BUTTON_BG: &str = "button.bg";
    /// Regular button background (hovered)
    pub const BUTTON_BG_HOVER: &str = "button.bg.hover";
    /// Regular button background (disabled)
    pub const BUTTON_BG_DISABLED: &str = "button.bg.disabled";
    /// Regular button background (pressed)
    pub const BUTTON_BG_PRESSED: &str = "button.bg.pressed";
    /// Regular button text
    pub const BUTTON_TEXT: &str = "button.txt";
    /// Regular button text (disabled)
    pub const BUTTON_TEXT_DISABLED: &str = "button.txt.disabled";

    // Primary ("default") buttons

    /// Primary button background
    pub const BUTTON_PRIMARY_BG: &str = "button.primary.bg";
    /// Primary button background (hovered)
    pub const BUTTON_PRIMARY_BG_HOVER: &str = "button.primary.bg.hover";
    /// Primary button background (disabled)
    pub const BUTTON_PRIMARY_BG_DISABLED: &str = "button.primary.bg.disabled";
    /// Primary button background (pressed)
    pub const BUTTON_PRIMARY_BG_PRESSED: &str = "button.primary.bg.pressed";
    /// Primary button text
    pub const BUTTON_PRIMARY_TEXT: &str = "button.primary.txt";
    /// Primary button text (disabled)
    pub const BUTTON_PRIMARY_TEXT_DISABLED: &str = "button.primary.txt.disabled";

    // Slider

    /// Background for slider
    pub const SLIDER_BG: &str = "slider.bg";
    /// Background for slider moving bar
    pub const SLIDER_BAR: &str = "slider.bar";
    /// Background for slider moving bar (disabled)
    pub const SLIDER_BAR_DISABLED: &str = "slider.bar.disabled";
    /// Background for slider text
    pub const SLIDER_TEXT: &str = "slider.text";
    /// Background for slider text (disabled)
    pub const SLIDER_TEXT_DISABLED: &str = "slider.text.disabled";

    // Checkbox

    /// Checkbox border around the checkmark
    pub const CHECKBOX_BORDER: &str = "checkbox.border";
    /// Checkbox border around the checkmark (hovered)
    pub const CHECKBOX_BORDER_HOVER: &str = "checkbox.border.hover";
    /// Checkbox border around the checkmark (disabled)
    pub const CHECKBOX_BORDER_DISABLED: &str = "checkbox.border.disabled";
    /// Checkbox check mark
    pub const CHECKBOX_MARK: &str = "checkbox.mark";
    /// Checkbox check mark (disabled)
    pub const CHECKBOX_MARK_DISABLED: &str = "checkbox.mark.disabled";
    /// Checkbox label text
    pub const CHECKBOX_TEXT: &str = "checkbox.text";
    /// Checkbox label text (disabled)
    pub const CHECKBOX_TEXT_DISABLED: &str = "checkbox.text.disabled";
}

pub(crate) fn update_theme(
    mut q_background: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, theme_bg) in q_background.iter_mut() {
            bg.0 = theme.color(theme_bg.0);
        }
    }
}

pub(crate) fn on_changed_background(
    ev: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(ev.target()) {
        bg.0 = theme.color(theme_bg.0);
    }
}

/// An observer which looks for changes to the `ThemeFontColor` component on an entity, and
/// propagates downward the text color to all participating text entities.
pub(crate) fn on_changed_font_color(
    ev: On<Insert, ThemeFontColor>,
    font_color: Query<&ThemeFontColor>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(token) = font_color.get(ev.target()) {
        let color = theme.color(token.0);
        commands
            .entity(ev.target())
            .insert(Propagate(TextColor(color)));
    }
}

/// Font asset paths
pub mod fonts {
    /// Default regular font path
    pub const REGULAR: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Regular.ttf";
    /// Regular italic font path
    pub const ITALIC: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Italic.ttf";
    /// Bold font path
    pub const BOLD: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-Bold.ttf";
    /// Bold italic font path
    pub const BOLD_ITALIC: &str = "embedded://bevy_feathers/assets/fonts/FiraSans-BoldItalic.ttf";
    /// Monospace font path
    pub const MONO: &str = "embedded://bevy_feathers/assets/fonts/FiraMono-Medium.ttf";
}

/// Size constants
pub mod size {
    use bevy_ui::Val;

    /// Common row size for buttons, sliders, spinners, etc.
    pub const ROW_HEIGHT: Val = Val::Px(22.0);
}

/// Constants for specifying which corners of a widget are rounded, used for segmented buttons
/// and control groups.
pub mod corners {
    use bevy_ui::{BorderRadius, Val};

    /// Allows specifying which corners are rounded and which are sharp. All rounded corners
    /// have the same radius. Not all combinations are supported, only the ones that make
    /// sense for a segmented buttons.
    ///
    /// A typical use case would be a segmented button consisting of 3 individual buttons in a
    /// row. In that case, you would have the leftmost button have rounded corners on the left,
    /// the right-most button have rounded corners on the right, and the center button have
    /// only sharp corners.
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub enum RoundedCorners {
        /// No corners are rounded.
        None,
        #[default]
        /// All corners are rounded.
        All,
        /// Top-left corner is rounded.
        TopLeft,
        /// Top-right corner is rounded.
        TopRight,
        /// Bottom-right corner is rounded.
        BottomRight,
        /// Bottom-left corner is rounded.
        BottomLeft,
        /// Top corners are rounded.
        Top,
        /// Right corners are rounded.
        Right,
        /// Bottom corners are rounded.
        Bottom,
        /// Left corners are rounded.
        Left,
    }

    impl RoundedCorners {
        /// Convert the `RoundedCorners` to a `BorderRadius` for use in a `Node`.
        pub fn to_border_radius(&self, radius: f32) -> BorderRadius {
            let radius = Val::Px(radius);
            let zero = Val::ZERO;
            match self {
                RoundedCorners::None => BorderRadius::all(zero),
                RoundedCorners::All => BorderRadius::all(radius),
                RoundedCorners::TopLeft => BorderRadius {
                    top_left: radius,
                    top_right: zero,
                    bottom_right: zero,
                    bottom_left: zero,
                },
                RoundedCorners::TopRight => BorderRadius {
                    top_left: zero,
                    top_right: radius,
                    bottom_right: zero,
                    bottom_left: zero,
                },
                RoundedCorners::BottomRight => BorderRadius {
                    top_left: zero,
                    top_right: zero,
                    bottom_right: radius,
                    bottom_left: zero,
                },
                RoundedCorners::BottomLeft => BorderRadius {
                    top_left: zero,
                    top_right: zero,
                    bottom_right: zero,
                    bottom_left: radius,
                },
                RoundedCorners::Top => BorderRadius {
                    top_left: radius,
                    top_right: radius,
                    bottom_right: zero,
                    bottom_left: zero,
                },
                RoundedCorners::Right => BorderRadius {
                    top_left: zero,
                    top_right: radius,
                    bottom_right: radius,
                    bottom_left: zero,
                },
                RoundedCorners::Bottom => BorderRadius {
                    top_left: zero,
                    top_right: zero,
                    bottom_right: radius,
                    bottom_left: radius,
                },
                RoundedCorners::Left => BorderRadius {
                    top_left: radius,
                    top_right: zero,
                    bottom_right: zero,
                    bottom_left: radius,
                },
            }
        }
    }
}
