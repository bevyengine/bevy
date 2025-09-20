//! A framework for theming.
use bevy_asset::AssetServer;
use bevy_color::{palettes, Color};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    hierarchy::{ChildOf, Children},
    lifecycle::Insert,
    observer::On,
    query::{Changed, With},
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Commands, Query, Res},
};
use bevy_log::warn_once;
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_text::{Font, TextColor, TextFont};
use bevy_ui::{BackgroundColor, BorderColor};
use smol_str::SmolStr;

use crate::{font_styles::InheritableFont, handle_or_path::HandleOrPath};

/// A design token for the theme. This serves as the lookup key for the theme properties.
#[derive(Clone, PartialEq, Eq, Hash, Reflect)]
pub struct ThemeToken(SmolStr);

impl ThemeToken {
    /// Construct a new [`ThemeToken`] from a [`SmolStr`].
    pub const fn new(text: SmolStr) -> Self {
        Self(text)
    }

    /// Construct a new [`ThemeToken`] from a static string.
    pub const fn new_static(text: &'static str) -> Self {
        Self(SmolStr::new_static(text))
    }
}

impl core::fmt::Display for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ThemeToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ThemeToken({:?})", self.0)
    }
}

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to colors.
    pub color: HashMap<ThemeToken, Color>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    pub fn color(&self, token: &ThemeToken) -> Color {
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
    pub fn set_color(&mut self, token: &str, color: Color) {
        self.0
            .color
            .insert(ThemeToken::new(SmolStr::new(token)), color);
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeFontColor(pub ThemeToken);

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
            bg.0 = theme.color(&theme_bg.0);
        }

        // Update all border colors
        for (mut border, theme_border) in q_border.iter_mut() {
            border.set_all(theme.color(&theme_border.0));
        }
    }
}

pub(crate) fn on_changed_background(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (&mut BackgroundColor, &ThemeBackgroundColor),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, theme_bg)) = q_background.get_mut(insert.entity) {
        bg.0 = theme.color(&theme_bg.0);
    }
}

pub(crate) fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor), Changed<ThemeBorderColor>>,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, theme_border)) = q_border.get_mut(insert.entity) {
        border.set_all(theme.color(&theme_border.0));
    }
}

/// An observer which looks for changes to the [`ThemeFontColor`] component on an entity, and
/// propagates downward the text color to all participating text entities.
pub(crate) fn on_changed_font_color(
    insert: On<Insert, ThemeFontColor>,
    q_font_color: Query<&ThemeFontColor>,
    q_children: Query<&Children>,
    q_themed_text: Query<(), With<ThemedText>>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok(token) = q_font_color.get(insert.entity) {
        let color = theme.color(&token.0);
        commands.insert_batch(
            q_children
                .iter_descendants(insert.entity)
                .filter(|text_entity| q_themed_text.contains(*text_entity))
                .map(|text_entity| (text_entity, TextColor(color)))
                .collect::<Vec<(Entity, TextColor)>>(),
        );
    }
}

/// An observer which looks for newly inserted or changed text nodes, and updates their
/// font and text color.
pub(crate) fn on_changed_text(
    insert: On<Insert, ThemedText>,
    q_font: Query<&InheritableFont>,
    q_font_color: Query<&ThemeFontColor>,
    q_parent: Query<&ChildOf>,
    assets: Res<AssetServer>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    let mut found_color = false;
    let mut found_font = false;
    for ancestor in q_parent.iter_ancestors(insert.entity) {
        if let Ok(token) = q_font_color.get(ancestor) {
            let color = theme.color(&token.0);
            commands.entity(insert.entity).insert(TextColor(color));
            found_color = true;
        }
        if let Ok(style) = q_font.get(ancestor)
            && let Some(font) = match style.font {
                HandleOrPath::Handle(ref h) => Some(h.clone()),
                HandleOrPath::Path(ref p) => Some(assets.load::<Font>(p)),
            }
        {
            commands.entity(insert.entity).insert(TextFont {
                font: font.clone(),
                font_size: style.font_size,
                ..Default::default()
            });
            found_font = true;
        }
        if found_color && found_font {
            break;
        }
    }
}
