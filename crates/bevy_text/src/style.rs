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
#[derive(Debug, Component, Clone, PartialEq, Reflect)]
#[reflect(Component, Clone, PartialEq)]
pub struct InheritedTextStyle<S: Component + Clone + PartialEq>(pub S);

/// The resolved text style for a text entity.
///
/// Updated by [`update_computed_text_styles`]
#[derive(Component, PartialEq, Debug, Default)]
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

fn update_computed_text_style(
    default_text_style: &DefaultTextStyle,
    mut style: Mut<ComputedTextStyle>,
    maybe_font: Option<&InheritedTextStyle<TextFont>>,
    maybe_color: Option<&InheritedTextStyle<TextColor>>,
    maybe_size: Option<&InheritedTextStyle<FontSize>>,
    maybe_line_height: Option<&InheritedTextStyle<LineHeight>>,
    maybe_smoothing: Option<&InheritedTextStyle<FontSmoothing>>,
) {
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
        || new_style.font_size != style.font_size
        || new_style.font_smoothing != style.font_smoothing
        || new_style.line_height != style.line_height
    {
        *style = new_style;
    } else {
        // bypass change detection, we don't need to update the layout if only the text color has changed
        style.bypass_change_detection().color = new_style.color;
    }
}

/// update computed styles
pub fn update_computed_text_styles(
    default_text_style: Res<DefaultTextStyle>,
    mut param_sets: ParamSet<(
        Query<(
            &mut ComputedTextStyle,
            Option<&InheritedTextStyle<TextFont>>,
            Option<&InheritedTextStyle<TextColor>>,
            Option<&InheritedTextStyle<FontSize>>,
            Option<&InheritedTextStyle<LineHeight>>,
            Option<&InheritedTextStyle<FontSmoothing>>,
        )>,
        Query<
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
    )>,
) {
    if default_text_style.is_changed() {
        for (style, maybe_font, maybe_color, maybe_size, maybe_line_height, maybe_smoothing) in
            param_sets.p0().iter_mut()
        {
            update_computed_text_style(
                default_text_style.as_ref(),
                style,
                maybe_font,
                maybe_color,
                maybe_size,
                maybe_line_height,
                maybe_smoothing,
            );
        }
    } else {
        for (style, maybe_font, maybe_color, maybe_size, maybe_line_height, maybe_smoothing) in
            param_sets.p1().iter_mut()
        {
            update_computed_text_style(
                default_text_style.as_ref(),
                style,
                maybe_font,
                maybe_color,
                maybe_size,
                maybe_line_height,
                maybe_smoothing,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::prelude::*;

    #[test]
    fn test_text_style_propagates_to_children() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.init_resource::<DefaultTextStyle>();
        app.add_systems(
            Update,
            (
                update_from_inherited_text_style_sources::<TextFont>,
                update_reparented_inherited_styles::<TextFont>,
                propagate_inherited_styles::<TextFont>,
                update_from_inherited_text_style_sources::<TextColor>,
                update_reparented_inherited_styles::<TextColor>,
                propagate_inherited_styles::<TextColor>,
                update_from_inherited_text_style_sources::<FontSize>,
                update_reparented_inherited_styles::<FontSize>,
                propagate_inherited_styles::<FontSize>,
                update_from_inherited_text_style_sources::<LineHeight>,
                update_reparented_inherited_styles::<LineHeight>,
                propagate_inherited_styles::<LineHeight>,
                update_from_inherited_text_style_sources::<FontSmoothing>,
                update_reparented_inherited_styles::<FontSmoothing>,
                propagate_inherited_styles::<FontSmoothing>,
                update_computed_text_styles,
            )
                .chain(),
        );

        let font_size = 99.;

        let propagator = app.world_mut().spawn_empty().id();

        let intermediate = app
            .world_mut()
            .spawn_empty()
            .insert(ChildOf(propagator))
            .id();
        let propagatee = app
            .world_mut()
            .spawn_empty()
            .insert((ComputedTextStyle::default(), ChildOf(intermediate)))
            .id();

        app.update();

        let style = app
            .world_mut()
            .query::<&ComputedTextStyle>()
            .get(app.world(), propagatee)
            .unwrap();

        assert_eq!(style.font_size, DefaultTextStyle::default().font_size);

        app.world_mut()
            .entity_mut(propagator)
            .insert(FontSize(font_size));

        app.update();

        let style = app
            .world_mut()
            .query::<&ComputedTextStyle>()
            .get(app.world(), propagatee)
            .unwrap();

        assert_eq!(style.font_size, font_size);
    }

    #[test]
    fn test_text_styles_update_when_text_reparented() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.init_resource::<DefaultTextStyle>();
        app.add_systems(
            Update,
            (
                update_from_inherited_text_style_sources::<TextFont>,
                update_reparented_inherited_styles::<TextFont>,
                propagate_inherited_styles::<TextFont>,
                update_from_inherited_text_style_sources::<TextColor>,
                update_reparented_inherited_styles::<TextColor>,
                propagate_inherited_styles::<TextColor>,
                update_from_inherited_text_style_sources::<FontSize>,
                update_reparented_inherited_styles::<FontSize>,
                propagate_inherited_styles::<FontSize>,
                update_from_inherited_text_style_sources::<LineHeight>,
                update_reparented_inherited_styles::<LineHeight>,
                propagate_inherited_styles::<LineHeight>,
                update_from_inherited_text_style_sources::<FontSmoothing>,
                update_reparented_inherited_styles::<FontSmoothing>,
                propagate_inherited_styles::<FontSmoothing>,
                update_computed_text_styles,
            )
                .chain(),
        );

        let source_1 = app.world_mut().spawn(FontSize(1.)).id();
        let source_2 = app.world_mut().spawn(FontSize(2.)).id();

        let target = app.world_mut().spawn(ComputedTextStyle::default()).id();

        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&ComputedTextStyle>()
                .get(app.world(), target)
                .unwrap()
                .font_size(),
            DefaultTextStyle::default().font_size
        );

        app.world_mut().entity_mut(target).insert(ChildOf(source_1));

        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&ComputedTextStyle>()
                .get(app.world(), target)
                .unwrap()
                .font_size(),
            1.
        );

        app.world_mut().entity_mut(target).insert(ChildOf(source_2));

        app.update();

        assert_eq!(
            app.world_mut()
                .query::<&ComputedTextStyle>()
                .get(app.world(), target)
                .unwrap()
                .font_size(),
            2.
        );
    }

    #[test]
    fn test_text_styles_update_when_default_text_style_changes() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.init_resource::<DefaultTextStyle>();
        app.add_systems(
            Update,
            (
                update_from_inherited_text_style_sources::<TextFont>,
                update_reparented_inherited_styles::<TextFont>,
                propagate_inherited_styles::<TextFont>,
                update_from_inherited_text_style_sources::<TextColor>,
                update_reparented_inherited_styles::<TextColor>,
                propagate_inherited_styles::<TextColor>,
                update_from_inherited_text_style_sources::<FontSize>,
                update_reparented_inherited_styles::<FontSize>,
                propagate_inherited_styles::<FontSize>,
                update_from_inherited_text_style_sources::<LineHeight>,
                update_reparented_inherited_styles::<LineHeight>,
                propagate_inherited_styles::<LineHeight>,
                update_from_inherited_text_style_sources::<FontSmoothing>,
                update_reparented_inherited_styles::<FontSmoothing>,
                propagate_inherited_styles::<FontSmoothing>,
                update_computed_text_styles,
            )
                .chain(),
        );

        let target_1 = app.world_mut().spawn(ComputedTextStyle::default()).id();

        let target_2 = app
            .world_mut()
            .spawn((FontSize(2.), ComputedTextStyle::default()))
            .id();

        let root_3 = app.world_mut().spawn_empty().id();

        let target_3 = app
            .world_mut()
            .spawn((ComputedTextStyle::default(), ChildOf(root_3)))
            .id();

        let root_4 = app.world_mut().spawn(FontSize(4.)).id();

        let target_4 = app
            .world_mut()
            .spawn((ComputedTextStyle::default(), ChildOf(root_4)))
            .id();

        let target_5 = app
            .world_mut()
            .spawn((FontSize(5.), ComputedTextStyle::default(), ChildOf(root_4)))
            .id();

        app.update();

        let default_default_text_style = DefaultTextStyle::default();
        for (target, expected_font_size) in [
            (target_1, default_default_text_style.font_size),
            (target_2, 2.),
            (target_3, default_default_text_style.font_size),
            (target_4, 4.),
            (target_5, 5.),
        ] {
            assert_eq!(
                *app.world_mut()
                    .query::<&ComputedTextStyle>()
                    .get(app.world(), target)
                    .unwrap(),
                ComputedTextStyle {
                    font_size: expected_font_size,
                    font: default_default_text_style.font.clone(),
                    color: default_default_text_style.color,
                    font_smoothing: default_default_text_style.font_smoothing,
                    line_height: default_default_text_style.line_height,
                }
            );
        }

        let mut default_text_style = app.world_mut().resource_mut::<DefaultTextStyle>();
        default_text_style.font_size = 17.;
        default_text_style.line_height = LineHeight::Px(99.);

        app.update();

        let default_default_text_style = DefaultTextStyle::default();
        for (target, expected_font_size) in [
            (target_1, 17.),
            (target_2, 2.),
            (target_3, 17.),
            (target_4, 4.),
            (target_5, 5.),
        ] {
            assert_eq!(
                *app.world_mut()
                    .query::<&ComputedTextStyle>()
                    .get(app.world(), target)
                    .unwrap(),
                ComputedTextStyle {
                    font_size: expected_font_size,
                    font: default_default_text_style.font.clone(),
                    color: default_default_text_style.color,
                    font_smoothing: default_default_text_style.font_smoothing,
                    line_height: LineHeight::Px(99.),
                }
            );
        }
    }

    #[test]
    fn test_override_inherited_text_style() {
        let mut app = App::new();
        app.add_schedule(Schedule::new(Update));
        app.init_resource::<DefaultTextStyle>();
        app.add_systems(
            Update,
            (
                update_from_inherited_text_style_sources::<TextFont>,
                update_reparented_inherited_styles::<TextFont>,
                propagate_inherited_styles::<TextFont>,
                update_from_inherited_text_style_sources::<TextColor>,
                update_reparented_inherited_styles::<TextColor>,
                propagate_inherited_styles::<TextColor>,
                update_from_inherited_text_style_sources::<FontSize>,
                update_reparented_inherited_styles::<FontSize>,
                propagate_inherited_styles::<FontSize>,
                update_from_inherited_text_style_sources::<LineHeight>,
                update_reparented_inherited_styles::<LineHeight>,
                propagate_inherited_styles::<LineHeight>,
                update_from_inherited_text_style_sources::<FontSmoothing>,
                update_reparented_inherited_styles::<FontSmoothing>,
                propagate_inherited_styles::<FontSmoothing>,
                update_computed_text_styles,
            )
                .chain(),
        );

        let root = app.world_mut().spawn(ComputedTextStyle::default()).id();

        let node_1 = app
            .world_mut()
            .spawn_empty()
            .insert((FontSize(1.), ComputedTextStyle::default(), ChildOf(root)))
            .id();

        let node_2 = app
            .world_mut()
            .spawn_empty()
            .insert((ComputedTextStyle::default(), ChildOf(node_1)))
            .id();

        let node_3 = app
            .world_mut()
            .spawn_empty()
            .insert((FontSize(2.), ComputedTextStyle::default(), ChildOf(node_2)))
            .id();

        let node_4 = app
            .world_mut()
            .spawn_empty()
            .insert((FontSize(3.), ComputedTextStyle::default(), ChildOf(node_3)))
            .id();

        let node_5 = app
            .world_mut()
            .spawn_empty()
            .insert((ComputedTextStyle::default(), ChildOf(node_4)))
            .id();

        app.update();

        for (target, expected_font_size) in [
            (root, DefaultTextStyle::default().font_size),
            (node_1, 1.),
            (node_2, 1.),
            (node_3, 2.),
            (node_4, 3.),
            (node_5, 3.),
        ] {
            assert_eq!(
                app.world_mut()
                    .query::<&ComputedTextStyle>()
                    .get(app.world(), target)
                    .unwrap()
                    .font_size,
                expected_font_size
            );
        }
    }
}
