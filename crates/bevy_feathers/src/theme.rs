//! A framework for theming.
use bevy_app::Propagate;
use bevy_color::{palettes, Color};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    lifecycle::Insert,
    observer::On,
    query::Changed,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::TextColor;
use bevy_ui::{BackgroundColor, BorderColor};

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<String, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
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
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub &'static str);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Copy)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub &'static str);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone, Copy)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeFontColor(pub &'static str);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct ThemedText;

pub(crate) fn update_theme(
    mut q_background: Query<(&mut BackgroundColor, &ThemeBackgroundColor)>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor)>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, theme_bg) in q_background.iter_mut() {
            bg.0 = theme.color(theme_bg.0);
        }

        // Update all border colors
        for (mut border, theme_border) in q_border.iter_mut() {
            border.set_all(theme.color(theme_border.0));
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

pub(crate) fn on_changed_border(
    ev: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, theme_border)) = q_border.get_mut(ev.target()) {
        border.set_all(theme.color(theme_border.0));
    }
}

/// An observer which looks for changes to the [`ThemeFontColor`] component on an entity, and
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
