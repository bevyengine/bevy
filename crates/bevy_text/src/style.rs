use crate::*;
use bevy_asset::Handle;
use bevy_color::Color;
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::*;
use bevy_ecs::relationship::Relationship;
use bevy_reflect::Reflect;

/// Text style
#[derive(Clone, PartialEq)]
pub struct TextStyle {
    /// The font used by a text entity when neither it nor any ancestor has a [`TextFont`] component.
    pub font: Handle<Font>,
    /// Default value
    pub font_size: f32,
    /// The color used by a text entity when neither it nor any ancestor has a [`TextColor`] component.
    pub color: Color,
    /// Default value
    pub font_smoothing: FontSmoothing,
    /// Default value
    pub line_height: LineHeight,
}

impl TextStyle {
    /// Returns the text style as a bundle of components
    pub fn bundle(&self) -> impl Bundle {
        (
            TextFont(self.font.clone()),
            FontSize(self.font_size),
            TextColor(self.color),
            self.font_smoothing,
            self.line_height,
        )
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            color: Color::WHITE,
            font_smoothing: FontSmoothing::default(),
            line_height: LineHeight::default(),
            font_size: FontSize::default().0,
        }
    }
}

/// Fallback text style used if a text entity and all its ancestors lack text styling components.
#[derive(Resource, Default, Clone, Deref, DerefMut)]
pub struct DefaultTextStyle(pub TextStyle);

/// Wrapper used to differentiate propagated text style compoonents
#[derive(Component, Clone, PartialEq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct InheritedTextStyle<S: Component + Clone + PartialEq>(pub S);

/// The resolved text style for a text entity.
///
/// Updated by [`update_computed_text_styles`]
#[derive(Component, PartialEq, Default)]
pub struct ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) font: Handle<Font>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    pub(crate) font_size: f32,
    /// The antialiasing method to use when rendering text.
    pub(crate) font_smoothing: FontSmoothing,
    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    pub(crate) line_height: LineHeight,
    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub(crate) color: Color,
}

impl ComputedTextStyle {
    /// The resolved font, taken from the nearest ancestor (including self) with a [`TextFont`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn font(&self) -> &Handle<Font> {
        &self.font
    }

    /// The resolved text color, taken from the nearest ancestor (including self) with a [`TextColor`],
    /// or from [`DefaultTextStyle`] if none is found.
    pub const fn color(&self) -> Color {
        self.color
    }

    /// The vertical height of a line of text, from the top of one line to the top of the
    /// next.
    pub const fn line_height(&self) -> LineHeight {
        self.line_height
    }

    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    pub const fn font_size(&self) -> f32 {
        self.font_size
    }

    /// The antialiasing method to use when rendering text.
    pub const fn font_smoothing(&self) -> FontSmoothing {
        self.font_smoothing
    }
}

/// update text style sources
pub fn update_from_inherited_text_style_sources<S: Component + Clone + PartialEq>(
    mut commands: Commands,
    changed_query: Query<(Entity, &S), Or<(Changed<S>, Without<InheritedTextStyle<S>>)>>,
    mut removed_styles: RemovedComponents<S>,
) {
    for (entity, source) in &changed_query {
        commands
            .entity(entity)
            .try_insert(InheritedTextStyle(source.clone()));
    }

    for removed_style in removed_styles.read() {
        if let Ok(mut commands) = commands.get_entity(removed_style) {
            commands.remove::<(InheritedTextStyle<S>, S)>();
        }
    }
}

/// update reparented and orphaned styles
pub fn update_reparented_inherited_styles<S: Component + Clone + PartialEq>(
    mut commands: Commands,
    moved: Query<
        (Entity, &ChildOf, Option<&InheritedTextStyle<S>>),
        (Changed<ChildOf>, Without<S>),
    >,
    parents: Query<&InheritedTextStyle<S>>,
    orphaned: Query<Entity, (With<InheritedTextStyle<S>>, Without<S>, Without<ChildOf>)>,
) {
    for (entity, parent, maybe_inherited) in &moved {
        if let Ok(inherited) = parents.get(parent.get()) {
            commands.entity(entity).try_insert(inherited.clone());
        } else if maybe_inherited.is_some() {
            commands.entity(entity).remove::<InheritedTextStyle<S>>();
        }
    }

    for orphan in &orphaned {
        commands.entity(orphan).remove::<InheritedTextStyle<S>>();
    }
}

/// propagate inherited styles
pub fn propagate_inherited_styles<S: Component + Clone + PartialEq>(
    mut commands: Commands,
    changed: Query<(&InheritedTextStyle<S>, &Children), Changed<InheritedTextStyle<S>>>,
    recurse: Query<(Option<&Children>, Option<&InheritedTextStyle<S>>), Without<S>>,
    mut removed: RemovedComponents<InheritedTextStyle<S>>,
    mut to_process: Local<Vec<(Entity, Option<InheritedTextStyle<S>>)>>,
) {
    // gather changed
    for (inherited, targets) in &changed {
        to_process.extend(
            targets
                .iter()
                .map(|target| (target, Some(inherited.clone()))),
        );
    }

    // and removed
    for entity in removed.read() {
        if let Ok((Some(targets), _)) = recurse.get(entity) {
            to_process.extend(targets.iter().map(|target| (target, None)));
        }
    }

    // propagate
    while let Some((entity, maybe_inherited)) = (*to_process).pop() {
        let Ok((maybe_targets, maybe_current)) = recurse.get(entity) else {
            continue;
        };

        if maybe_current == maybe_inherited.as_ref() {
            continue;
        }

        if let Some(targets) = maybe_targets {
            to_process.extend(
                targets
                    .iter()
                    .map(|target| (target, maybe_inherited.clone())),
            );
        }

        if let Some(inherited) = maybe_inherited {
            commands.entity(entity).try_insert(inherited.clone());
        } else {
            commands.entity(entity).remove::<InheritedTextStyle<S>>();
        }
    }
}

/// update computed styles
pub fn update_computed_text_styles(
    default_text_style: Res<DefaultTextStyle>,
    mut query: Query<
        (
            &mut ComputedTextStyle,
            Option<&InheritedTextStyle<TextFont>>,
            Option<&InheritedTextStyle<TextColor>>,
            Option<&InheritedTextStyle<FontSize>>,
            Option<&InheritedTextStyle<LineHeight>>,
            Option<&InheritedTextStyle<FontSmoothing>>,
        ),
        Or<(
            Changed<InheritedTextStyle<TextFont>>,
            Changed<InheritedTextStyle<TextColor>>,
            Changed<InheritedTextStyle<FontSize>>,
            Changed<InheritedTextStyle<LineHeight>>,
            Changed<InheritedTextStyle<FontSmoothing>>,
            Added<ComputedTextStyle>,
        )>,
    >,
) {
    for (mut style, maybe_font, maybe_color, maybe_size, maybe_line_height, maybe_smoothing) in
        query.iter_mut()
    {
        let new_style = ComputedTextStyle {
            font: maybe_font
                .map_or(&default_text_style.font, |font| &font.0 .0)
                .clone(),
            color: maybe_color
                .map(|t| t.0 .0)
                .unwrap_or(default_text_style.color),
            font_size: maybe_size.map_or(default_text_style.font_size, |size| size.0 .0),
            font_smoothing: maybe_smoothing
                .map(|s| s.0)
                .unwrap_or(default_text_style.font_smoothing),
            line_height: maybe_line_height
                .map(|l| l.0)
                .unwrap_or(default_text_style.line_height),
        };

        if new_style.font != style.font
            && new_style.font_size != style.font_size
            && new_style.font_smoothing != style.font_smoothing
            && new_style.line_height != style.line_height
        {
            *style = new_style;
        } else {
            // bypass change detection, we don't need to do any updates if only the text color has changed
            style.bypass_change_detection().color = new_style.color;
        }
    }
}
