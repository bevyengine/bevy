use alloc::string::String;

use bevy_app::{Plugin, PostUpdate, PreUpdate, PropagateOver};
use bevy_camera::visibility::Visibility;
use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::{Remove, RemovedComponents},
    observer::On,
    query::{Added, Changed, Has, Or, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
};
use bevy_picking::{cursor::EntityCursor, Pickable, PickingSystems};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, FontSourceTemplate, FontWeight, LineBreak, TextCursorStyle, TextFont, TextLayout,
    TextReadWriteMode,
};
use bevy_ui::{
    px, widget::Text, AlignItems, BorderRadius, ComputedNode, Display, InteractionDisabled,
    JustifyContent, Node, PositionType, UiGlobalTransform, UiRect, UiSystems,
};
use bevy_ui_widgets::TextInput;

use crate::{
    constants::{fonts, size},
    focus::FocusWithinIndicator,
    font_styles::InheritableFont,
    theme::{
        InheritableThemeTextColor, SurfaceLevel, ThemeBackgroundColor, ThemeContext,
        ThemeTextColor, ThemedText, UiTheme,
    },
    tokens,
};

/// Decorative frame around a text input widget. This is a separate entity to allow icons
/// (such as "search" or "clear") to be inserted adjacent to the input.
///
/// This is spawnable by inheriting it as a "scene component".
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersTextInputContainer;

impl FeathersTextInputContainer {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                height: size::ROW_HEIGHT,
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect {
                    right: px(3.0),
                    left: px(3.0),
                },
                flex_grow: 1.0,
                border_radius: {BorderRadius::all(px(4.0))},
                column_gap: px(4),
            }
            FeathersTextInputContainer
            FocusWithinIndicator
            ThemeBackgroundColor(tokens::TEXT_INPUT_BG)
            InheritableThemeTextColor(tokens::TEXT_INPUT_TEXT)
            InheritableFont {
                font: FontSourceTemplate::Families(fonts::SANS),
                font_size: size::COMPACT_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}

/// Scene function to spawn a text input. For proper styling, this should be enclosed by a [`FeathersTextInputContainer`].
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersTextInputProps`].
///
/// ```ignore
/// @FeathersTextInputContainer
/// Children [
///     :FeathersTextInput
/// ]
/// ```
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
pub struct FeathersTextInput;

/// Props used to construct the [`FeathersTextInput`] scene.
#[derive(Default, Clone)]
pub struct FeathersTextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
}

impl FeathersTextInput {
    fn scene(props: FeathersTextInputProps) -> impl Scene {
        bsn! {
            Node {
                flex_grow: {
                    if props.visible_width.is_some() {
                        0_f32
                    } else {
                        1_f32
                    }
                } ,
            }
            FeathersTextInput
            TextInput
            EditableText {
                cursor_width: 0.3,
                visible_width: {props.visible_width},
                max_characters: {props.max_characters},
            }
            ThemedText
            TextLayout {
                linebreak: LineBreak::NoWrap,
            }
            TextFont {
                font: FontSourceTemplate::Families(fonts::SANS),
                font_size: size::COMPACT_FONT,
                weight: FontWeight::NORMAL,
            }
            PropagateOver<TextFont>
            EntityCursor::System(bevy_window::SystemCursorIcon::Text)
            TextCursorStyle
        }
    }
}

/// Hint text shown by a [`FeathersTextInput`] while it is empty.
///
/// The hint is displayed by a sibling entity, so the input must have a parent such as
/// [`FeathersTextInputContainer`].
///
/// ```ignore
/// @FeathersTextInputContainer
/// Children [
///     @FeathersTextInput
///     TextInputPlaceholder("Search")
/// ]
/// ```
#[derive(Component, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct TextInputPlaceholder(pub String);

/// The sibling entity that displays an input's [`TextInputPlaceholder`].
///
/// A sibling rather than a child, because a UI node with children is not measured, which would
/// collapse the size of the input.
#[derive(Component)]
struct TextInputPlaceholderLabel(Entity);

fn sync_text_input_placeholders(
    q_inputs: Query<
        (
            Entity,
            &TextInputPlaceholder,
            &EditableText,
            &TextFont,
            &ChildOf,
            Option<&TextInputPlaceholderLabel>,
        ),
        Or<(
            Changed<TextInputPlaceholder>,
            Changed<EditableText>,
            Changed<TextFont>,
            Changed<ComputedNode>,
        )>,
    >,
    q_layout: Query<(&ComputedNode, &UiGlobalTransform)>,
    mut q_labels: Query<
        (&mut Node, &mut Text, &mut TextFont, &mut Visibility),
        Without<EditableText>,
    >,
    mut commands: Commands,
) {
    for (input, placeholder, editable_text, font, child_of, label) in q_inputs.iter() {
        let visibility = if editable_text.value().is_empty() && !editable_text.is_composing() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let left = match (q_layout.get(input), q_layout.get(child_of.parent())) {
            (Ok(input_layout), Ok(parent_layout)) => px(logical_left(input_layout, parent_layout)),
            _ => px(0),
        };
        match label.and_then(|label| q_labels.get_mut(label.0).ok()) {
            Some((mut node, mut text, mut label_font, mut label_visibility)) => {
                if node.left != left {
                    node.left = left;
                }
                if text.0 != placeholder.0 {
                    text.0.clone_from(&placeholder.0);
                }
                label_font.set_if_neq(font.clone());
                label_visibility.set_if_neq(visibility);
            }
            None => {
                let label = commands
                    .spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            left,
                            ..Default::default()
                        },
                        Text(placeholder.0.clone()),
                        font.clone(),
                        ThemeTextColor(tokens::TEXT_INPUT_TEXT_DISABLED),
                        TextLayout {
                            linebreak: LineBreak::NoWrap,
                            ..Default::default()
                        },
                        Pickable::IGNORE,
                        visibility,
                        ChildOf(child_of.parent()),
                    ))
                    .id();
                commands
                    .entity(input)
                    .insert(TextInputPlaceholderLabel(label));
            }
        }
    }
}

fn logical_left(
    (input, input_transform): (&ComputedNode, &UiGlobalTransform),
    (parent, parent_transform): (&ComputedNode, &UiGlobalTransform),
) -> f32 {
    let input_left = input_transform.translation.x - input.size.x / 2.0;
    let parent_left = parent_transform.translation.x - parent.size.x / 2.0;
    (input_left - parent_left) * input.inverse_scale_factor
}

fn remove_text_input_placeholder_label(
    remove: On<Remove<TextInputPlaceholder>>,
    q_labels: Query<&TextInputPlaceholderLabel>,
    mut commands: Commands,
) {
    if let Ok(label) = q_labels.get(remove.entity) {
        commands.entity(label.0).try_despawn();
        commands
            .entity(remove.entity)
            .try_remove::<TextInputPlaceholderLabel>();
    }
}

fn update_text_cursor_color(
    mut q_text_input: Query<(&mut TextCursorStyle, Option<&ThemeContext>), With<FeathersTextInput>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for (mut cursor_style, theme_context) in q_text_input.iter_mut() {
            let context = theme_context.map(|tc| tc.0).unwrap_or(SurfaceLevel::Base);
            cursor_style.color = theme.context_color(&tokens::TEXT_INPUT_CURSOR, context);
            cursor_style.selection_color =
                theme.context_color(&tokens::TEXT_INPUT_SELECTION, context);
            cursor_style.unfocused_selection_color =
                theme.context_color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED, context);
        }
    }
}

fn update_text_input_styles(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        (With<FeathersTextInput>, Added<InteractionDisabled>),
    >,
    mut commands: Commands,
) {
    for (input_ent, disabled, font_color) in q_inputs.iter() {
        set_text_input_styles(input_ent, disabled, font_color, &mut commands);
        commands
            .entity(input_ent)
            .insert(TextReadWriteMode::ReadOnly);
    }
}

fn update_text_input_styles_remove(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        With<FeathersTextInput>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((input_ent, disabled, font_color)) = q_inputs.get(ent) {
            set_text_input_styles(input_ent, disabled, font_color, &mut commands);
            commands
                .entity(input_ent)
                .insert(TextReadWriteMode::Editable);
        }
    });
}

fn set_text_input_styles(
    input_ent: Entity,
    disabled: bool,
    font_color: &InheritableThemeTextColor,
    commands: &mut Commands,
) {
    let font_color_token = match disabled {
        true => tokens::TEXT_INPUT_TEXT_DISABLED,
        false => tokens::TEXT_INPUT_TEXT,
    };

    let cursor_shape = match disabled {
        true => bevy_window::SystemCursorIcon::NotAllowed,
        false => bevy_window::SystemCursorIcon::Text,
    };

    // Change font color
    if font_color.0 != font_color_token {
        commands
            .entity(input_ent)
            .insert(InheritableThemeTextColor(font_color_token));
    }

    // Change cursor shape
    commands
        .entity(input_ent)
        .insert(EntityCursor::System(cursor_shape));
}

/// Plugin which registers the systems for updating the text input styles.
pub struct TextInputPlugin;

impl Plugin for TextInputPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            (
                update_text_cursor_color,
                update_text_input_styles,
                update_text_input_styles_remove,
            )
                .in_set(PickingSystems::Last),
        )
        .add_systems(
            PostUpdate,
            sync_text_input_placeholders.before(UiSystems::Prepare),
        )
        .add_observer(remove_text_input_placeholder_label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, Update};
    use bevy_ecs::hierarchy::Children;
    use bevy_math::{Affine2, Vec2};

    fn spawn_input(app: &mut App, text: &str) -> (Entity, Entity) {
        let container = app.world_mut().spawn_empty().id();
        let input = app
            .world_mut()
            .spawn((
                EditableText::new(text),
                TextFont::default(),
                TextInputPlaceholder("Search".into()),
                ChildOf(container),
            ))
            .id();
        app.update();
        (container, input)
    }

    fn input_app() -> App {
        let mut app = App::new();
        app.add_systems(Update, sync_text_input_placeholders)
            .add_observer(remove_text_input_placeholder_label);
        app
    }

    fn label_of(app: &App, input: Entity) -> Entity {
        app.world()
            .get::<TextInputPlaceholderLabel>(input)
            .unwrap()
            .0
    }

    #[test]
    fn label_is_a_sibling_showing_the_placeholder() {
        let mut app = input_app();
        let (container, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        assert_eq!(
            app.world().get::<ChildOf>(label).unwrap().parent(),
            container
        );
        assert!(app.world().get::<Children>(input).is_none());
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Search");
        assert_eq!(
            app.world().get::<Visibility>(label),
            Some(&Visibility::Inherited)
        );
        assert_eq!(app.world().get::<Pickable>(label), Some(&Pickable::IGNORE));
    }

    #[test]
    fn hidden_while_there_is_text() {
        let mut app = input_app();
        let (_, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        let set_text = |app: &mut App, text| {
            let mut editable = app.world_mut().get_mut::<EditableText>(input).unwrap();
            editable.editor_mut().set_text(text);
            app.update();
            *app.world().get::<Visibility>(label).unwrap()
        };
        assert_eq!(set_text(&mut app, "x"), Visibility::Hidden);
        assert_eq!(set_text(&mut app, ""), Visibility::Inherited);
    }

    #[test]
    fn hidden_when_spawned_with_text() {
        let mut app = input_app();
        let (_, input) = spawn_input(&mut app, "x");
        let label = label_of(&app, input);
        assert_eq!(
            app.world().get::<Visibility>(label),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn follows_placeholder_and_font_changes() {
        let mut app = input_app();
        let (_, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        app.world_mut().entity_mut(input).insert((
            TextInputPlaceholder("Filter".into()),
            TextFont {
                font_size: bevy_text::FontSize::Px(30.0),
                ..Default::default()
            },
        ));
        app.update();
        assert_eq!(label_of(&app, input), label);
        assert_eq!(app.world().get::<Text>(label).unwrap().0, "Filter");
        assert_eq!(
            app.world().get::<TextFont>(label).unwrap().font_size,
            bevy_text::FontSize::Px(30.0)
        );
    }

    #[test]
    fn label_lines_up_with_the_input() {
        let mut app = input_app();
        let (container, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        let layout = |x: f32, width: f32| {
            (
                ComputedNode {
                    size: Vec2::new(width, 20.0),
                    inverse_scale_factor: 0.5,
                    ..Default::default()
                },
                UiGlobalTransform::from(Affine2::from_translation(Vec2::new(x, 0.0))),
            )
        };
        app.world_mut()
            .entity_mut(container)
            .insert(layout(100.0, 200.0));
        app.world_mut()
            .entity_mut(input)
            .insert(layout(120.0, 160.0));
        app.update();
        assert_eq!(app.world().get::<Node>(label).unwrap().left, px(20));
    }

    #[test]
    fn removing_the_placeholder_removes_the_label() {
        let mut app = input_app();
        let (_, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        app.world_mut()
            .entity_mut(input)
            .remove::<TextInputPlaceholder>();
        app.update();
        assert!(app.world().get_entity(label).is_err());
        assert!(app
            .world()
            .get::<TextInputPlaceholderLabel>(input)
            .is_none());
    }

    #[test]
    fn despawning_the_input_removes_the_label() {
        let mut app = input_app();
        let (_, input) = spawn_input(&mut app, "");
        let label = label_of(&app, input);
        app.world_mut().entity_mut(input).despawn();
        app.update();
        assert!(app.world().get_entity(label).is_err());
    }
}
