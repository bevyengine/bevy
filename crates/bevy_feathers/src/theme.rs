//! A framework for theming.
use bevy_app::{Propagate, PropagateOver};
use bevy_color::{palettes, Color};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
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
use smol_str::SmolStr;

/// Indicates the type of surface context for computing color assignments
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Reflect, Default)]
#[reflect(Default, Debug)]
pub enum SurfaceLevel {
    /// The base layer - window, panes and subpanes
    #[default]
    Base,
    /// A raised layer - pane and subpane bodies
    Higher,
    /// The highest level - groups
    Highest,
    /// An overlay such as a dialog or menu
    Floating,
}

/// Component which is placed on a container and which propagates to children. This is used
/// to modify the color assignments based on the background container.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default, Reflect)]
#[reflect(Default, Debug)]
pub struct ThemeContext(pub SurfaceLevel);

/// A design token for the theme. This serves as the lookup key for the theme properties.
#[derive(Clone, PartialEq, Eq, Hash, Reflect, Default)]
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

/// A semantic-level token for the theme. Semantic tokens represent general classes of colors,
/// such as "surface.window" or "border.selected.accent", rather than specific color assignments.
/// Contextual theming happens at the semantic level, so (for example) lightening the token
/// for "fill.solid.default" affects all color assignments that map to that token.
#[derive(Clone, PartialEq, Eq, Hash, Reflect, Default)]
pub struct SemanticToken(SmolStr);

impl SemanticToken {
    /// Construct a new [`SemanticToken`] from a [`SmolStr`].
    pub const fn new(text: SmolStr) -> Self {
        Self(text)
    }

    /// Construct a new [`SemanticToken`] from a static string.
    pub const fn new_static(text: &'static str) -> Self {
        Self(SmolStr::new_static(text))
    }
}

impl core::fmt::Display for SemanticToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for SemanticToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SemanticToken({:?})", self.0)
    }
}

/// A collection of properties that make up a theme.
#[derive(Default, Clone, Reflect, Debug)]
#[reflect(Default, Debug)]
pub struct ThemeProps {
    /// Map of design tokens to semantic tokens.
    pub token_assignments: HashMap<ThemeToken, SemanticToken>,
    /// Map of semantic tokens to colors.
    pub semantic_base: HashMap<SemanticToken, Color>,
    /// Map of semantic tokens + context to colors.
    pub semantic_overrides: HashMap<SurfaceLevel, HashMap<SemanticToken, Color>>,
    // Other style property types to be added later.
}

/// The currently selected user interface theme. Overwriting this resource changes the theme.
#[derive(Resource, Default, Reflect, Debug)]
#[reflect(Resource, Default, Debug)]
pub struct UiTheme(pub ThemeProps);

impl UiTheme {
    /// Lookup a color by design token. If the theme does not have an entry for that token,
    /// logs a warning and returns an error color.
    ///
    /// This version does not take context into account, and is mainly left here for
    /// backwards-compatibility reasons.
    pub fn color(&self, token: &ThemeToken) -> Color {
        self.context_color(token, SurfaceLevel::Base)
    }

    /// Lookup a color by design token and context. If the combination of token and context is
    /// not found, then use the base map. If the theme does not have an entry for that
    /// token, logs a warning and returns an error color.
    pub fn context_color(&self, token: &ThemeToken, context: SurfaceLevel) -> Color {
        let Some(semantic_token) = self.0.token_assignments.get(token) else {
            warn_once!("Theme color {} not found.", token);
            // Return a bright obnoxious color to make the error obvious.
            return palettes::basic::FUCHSIA.into();
        };
        if let Some(color) = self
            .0
            .semantic_overrides
            .get(&context)
            .and_then(|m| m.get(semantic_token))
        {
            return *color;
        }
        if let Some(color) = self.0.semantic_base.get(semantic_token) {
            return *color;
        }
        warn_once!("Theme semantic color {:?} not found.", semantic_token);
        palettes::basic::FUCHSIA.into()
    }
}

/// Component which causes the background color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[require(BackgroundColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBackgroundColor(pub ThemeToken);

/// Component which causes the border color of an entity to be set based on a theme color.
/// Only supports setting all borders to the same color.
#[derive(Component, Clone, Default)]
#[require(BorderColor)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
pub struct ThemeBorderColor(pub ThemeToken);

/// Component which causes the inherited text color of an entity to be set based on a theme color.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct InheritableThemeTextColor(pub ThemeToken);

/// Component which causes the color of a text span to be set based on a theme color. Unlike
/// [`InheritableThemeTextColor`], this can work when set directly on the text span entity, and is
/// not inherited.
// TODO: This is necessary because an entity with Propagate doesn't update itself, only its
// descendants.
#[derive(Component, Clone, Default)]
#[component(immutable)]
#[derive(Reflect)]
#[reflect(Component, Clone)]
#[require(ThemedText, PropagateOver::<TextColor>)]
pub struct ThemeTextColor(pub ThemeToken);

/// A marker component that is used to indicate that the text entity wants to opt-in to using
/// inherited text styles.
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
pub struct ThemedText;

pub(crate) fn update_theme(
    mut q_background: Query<(
        &mut BackgroundColor,
        &ThemeBackgroundColor,
        Option<&ThemeContext>,
    )>,
    mut q_border: Query<(&mut BorderColor, &ThemeBorderColor, Option<&ThemeContext>)>,
    mut q_text_color: Query<(&mut TextColor, &ThemeTextColor, Option<&ThemeContext>)>,
    q_context_changed: Query<Entity, Changed<ThemeContext>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        // Update all background colors
        for (mut bg, ThemeBackgroundColor(token), ctx) in q_background.iter_mut() {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            bg.0 = theme.context_color(token, context);
        }

        // Update all border colors
        for (mut border, ThemeBorderColor(token), ctx) in q_border.iter_mut() {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            border.set_all(theme.context_color(token, context));
        }

        // Update all direct text span colors
        for (mut text_color, ThemeTextColor(token), ctx) in q_text_color.iter_mut() {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            text_color.0 = theme.context_color(token, context);
        }
    }

    // Because propagation happens after observers run, do a fix-up pass
    for ent in q_context_changed.iter() {
        // Update the background color
        if let Ok((mut bg, ThemeBackgroundColor(token), ctx)) = q_background.get_mut(ent) {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            bg.0 = theme.context_color(token, context);
        }

        // Update the border color
        if let Ok((mut border, ThemeBorderColor(token), ctx)) = q_border.get_mut(ent) {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            border.set_all(theme.context_color(token, context));
        }

        // Update the direct text span color
        if let Ok((mut text_color, ThemeTextColor(token), ctx)) = q_text_color.get_mut(ent) {
            let context = ctx.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            text_color.0 = theme.context_color(token, context);
        }
    }
}

pub(crate) fn on_changed_background(
    insert: On<Insert, ThemeBackgroundColor>,
    mut q_background: Query<
        (
            &mut BackgroundColor,
            &ThemeBackgroundColor,
            Option<&ThemeContext>,
        ),
        Changed<ThemeBackgroundColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut bg, ThemeBackgroundColor(token), theme_context)) =
        q_background.get_mut(insert.entity)
    {
        let context = theme_context.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
        bg.0 = theme.context_color(token, context);
    }
}

pub(crate) fn on_changed_border(
    insert: On<Insert, ThemeBorderColor>,
    mut q_border: Query<
        (&mut BorderColor, &ThemeBorderColor, Option<&ThemeContext>),
        Changed<ThemeBorderColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut border, ThemeBorderColor(token), theme_context)) =
        q_border.get_mut(insert.entity)
    {
        let context = theme_context.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
        border.set_all(theme.context_color(token, context));
    }
}

pub(crate) fn on_changed_text_color(
    insert: On<Insert, ThemeTextColor>,
    mut q_span: Query<
        (&mut TextColor, &ThemeTextColor, Option<&ThemeContext>),
        Changed<ThemeTextColor>,
    >,
    theme: Res<UiTheme>,
) {
    // Update background colors where the design token has changed.
    if let Ok((mut text_color, ThemeTextColor(token), theme_context)) =
        q_span.get_mut(insert.entity)
    {
        let context = theme_context.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
        text_color.0 = theme.context_color(token, context);
    }
}

/// An observer which looks for changes to the [`InheritableThemeTextColor`] component on an entity,
/// and propagates downward the text color to all participating text entities.
pub(crate) fn on_changed_font_color(
    insert: On<Insert, InheritableThemeTextColor>,
    font_color: Query<(&InheritableThemeTextColor, Option<&ThemeContext>)>,
    theme: Res<UiTheme>,
    mut commands: Commands,
) {
    if let Ok((InheritableThemeTextColor(token), theme_context)) = font_color.get(insert.entity) {
        let context = theme_context.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
        let color = theme.context_color(token, context);
        commands
            .entity(insert.entity)
            .insert(Propagate(TextColor(color)));
    }
}
