use bevy_app::{Plugin, PreUpdate, PropagateOver};
use bevy_asset::AssetServer;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    lifecycle::RemovedComponents,
    observer::On,
    query::{Added, Changed, Has, Or, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input_focus::{tab_navigation::TabIndex, FocusLost};
use bevy_picking::PickingSystems;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, FontSource, FontWeight, LineBreak, TextCursorStyle, TextEdit, TextEditChange,
    TextFont, TextLayout,
};
use bevy_ui::{
    px, AlignItems, BorderRadius, Display, InteractionDisabled, JustifyContent, Node, UiRect,
};
use bevy_ui_widgets::ValueChange;

use crate::{
    constants::{fonts, size},
    cursor::EntityCursor,
    focus::FocusWithinIndicator,
    font_styles::InheritableFont,
    theme::{InheritableThemeTextColor, ThemeBackgroundColor, ThemedText, UiTheme},
    tokens,
};

/// Decorative frame around a text input widget. This is a separate entity to allow adornments
/// (such as "search" or "clear" icons) to be inserted adjacent to the input.
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
                font: fonts::REGULAR,
                font_size: size::COMPACT_FONT,
                weight: FontWeight::NORMAL,
            }
        }
    }
}

/// Scene function to spawn a decorated text input.
///
/// This is spawnable by inheriting it as a "scene component" with optional [`FeathersTextInputProps`].
///
/// ```ignore
/// @FeathersTextInput
/// ```
///
/// Read or replace the text through [`TextInputValue`] on the widget root. User edits emit
/// [`ValueChange<String>`] from the root, with a final event when the input loses focus.
#[derive(SceneComponent, Default, Clone)]
#[scene(FeathersTextInputProps)]
#[derive(Reflect)]
#[reflect(Component, Default, Clone)]
#[require(TextInputValue)]
pub struct FeathersTextInput;

/// The text value exposed by a [`FeathersTextInput`] on its root entity.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[component(immutable)]
#[reflect(Component, Default, Clone, PartialEq)]
pub struct TextInputValue(pub String);

/// Props used to construct the [`FeathersTextInput`] scene.
pub struct FeathersTextInputProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
    /// Adornments to place before the editable text.
    pub leading_adornments: Box<dyn SceneList>,
    /// Adornments to place after the editable text.
    pub trailing_adornments: Box<dyn SceneList>,
}

impl Default for FeathersTextInputProps {
    fn default() -> Self {
        Self {
            visible_width: None,
            max_characters: None,
            leading_adornments: Box::new(bsn_list!()),
            trailing_adornments: Box::new(bsn_list!()),
        }
    }
}

impl FeathersTextInput {
    fn scene(props: FeathersTextInputProps) -> impl Scene {
        bsn! {
            @FeathersTextInputContainer
            FeathersTextInput
            Children [{props.leading_adornments}, (
                @FeathersTextInputInner {
                    @visible_width: {props.visible_width},
                    @max_characters: {props.max_characters},
                }
            ), {props.trailing_adornments}]
        }
    }
}

fn update_text_input_value(
    q_inputs: Query<(Entity, &TextInputValue), (With<FeathersTextInput>, Changed<TextInputValue>)>,
    q_children: Query<&Children>,
    mut q_inner: Query<&mut EditableText, With<FeathersTextInputInner>>,
) {
    for (entity, value) in &q_inputs {
        let Some(inner) = q_children
            .iter_descendants(entity)
            .find(|entity| q_inner.contains(*entity))
        else {
            continue;
        };
        let mut editable_text = q_inner.get_mut(inner).unwrap();
        if editable_text.value() != &value.0 {
            editable_text.queue_edit(TextEdit::SelectAll);
            editable_text.queue_edit(TextEdit::Insert(value.0.clone().into()));
        }
    }
}

fn text_input_on_change(
    change: On<TextEditChange>,
    q_parents: Query<&ChildOf>,
    q_inner: Query<&EditableText, With<FeathersTextInputInner>>,
    q_inputs: Query<&TextInputValue, With<FeathersTextInput>>,
    mut commands: Commands,
) {
    let Ok(editable_text) = q_inner.get(change.event_target()) else {
        return;
    };
    let Some(root) = q_parents
        .iter_ancestors(change.event_target())
        .find(|entity| q_inputs.contains(*entity))
    else {
        return;
    };
    let value = editable_text.value().to_string();
    if q_inputs.get(root).is_ok_and(|current| current.0 != value) {
        commands.entity(root).insert(TextInputValue(value.clone()));
        commands.trigger(ValueChange {
            source: root,
            value,
            is_final: false,
        });
    }
}

fn text_input_on_focus_lost(
    focus_lost: On<FocusLost>,
    q_parents: Query<&ChildOf>,
    q_inner: Query<&EditableText, With<FeathersTextInputInner>>,
    q_inputs: Query<(), With<FeathersTextInput>>,
    mut commands: Commands,
) {
    let Ok(editable_text) = q_inner.get(focus_lost.event_target()) else {
        return;
    };
    if let Some(root) = q_parents
        .iter_ancestors(focus_lost.event_target())
        .find(|entity| q_inputs.contains(*entity))
    {
        commands.trigger(ValueChange {
            source: root,
            value: editable_text.value().to_string(),
            is_final: true,
        });
    }
}

fn update_text_input_disabled(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>),
        (
            With<FeathersTextInput>,
            Or<(Added<FeathersTextInput>, Added<InteractionDisabled>)>,
        ),
    >,
    q_children: Query<&Children>,
    q_inner: Query<(), With<FeathersTextInputInner>>,
    mut commands: Commands,
) {
    for (entity, disabled) in &q_inputs {
        if let Some(input) = q_children
            .iter_descendants(entity)
            .find(|entity| q_inner.contains(*entity))
        {
            if disabled {
                commands.entity(input).insert(InteractionDisabled);
            } else {
                commands.entity(input).remove::<InteractionDisabled>();
            }
        }
    }
}

fn update_text_input_disabled_remove(
    q_inputs: Query<(), With<FeathersTextInput>>,
    q_children: Query<&Children>,
    q_inner: Query<(), With<FeathersTextInputInner>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    for entity in removed_disabled
        .read()
        .filter(|entity| q_inputs.contains(*entity))
    {
        if let Some(input) = q_children
            .iter_descendants(entity)
            .find(|entity| q_inner.contains(*entity))
        {
            commands.entity(input).remove::<InteractionDisabled>();
        }
    }
}

/// The editable text entity used internally by Feathers text-based controls.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[scene(FeathersTextInputInnerProps)]
#[reflect(Component, Default, Clone)]
pub(crate) struct FeathersTextInputInner;

/// Props used to construct a [`FeathersTextInputInner`] scene.
#[derive(Default)]
pub(crate) struct FeathersTextInputInnerProps {
    /// Visible width
    pub visible_width: Option<f32>,
    /// Max characters
    pub max_characters: Option<usize>,
}

impl FeathersTextInputInner {
    fn scene(props: FeathersTextInputInnerProps) -> impl Scene {
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
            FeathersTextInputInner
            EditableText {
                cursor_width: 0.3,
                visible_width: {props.visible_width},
                max_characters: {props.max_characters},
            }
            ThemedText
            TextLayout {
                linebreak: LineBreak::NoWrap,
            }
            TabIndex(0)
            template(|ctx| {
                Ok(TextFont {
                    font: FontSource::Handle(ctx.resource::<AssetServer>().load(fonts::REGULAR)),
                    font_size: size::COMPACT_FONT,
                    weight: FontWeight::NORMAL,
                    ..Default::default()
                })
            })
            PropagateOver<TextFont>
            EntityCursor::System(bevy_window::SystemCursorIcon::Text)
            TextCursorStyle::default()
            on(text_input_on_change)
            on(text_input_on_focus_lost)
        }
    }
}

fn update_text_cursor_color(
    mut q_text_input: Query<&mut TextCursorStyle, With<FeathersTextInputInner>>,
    theme: Res<UiTheme>,
) {
    if theme.is_changed() {
        for mut cursor_style in q_text_input.iter_mut() {
            cursor_style.color = theme.color(&tokens::TEXT_INPUT_CURSOR);
            cursor_style.selection_color = theme.color(&tokens::TEXT_INPUT_SELECTION);
            cursor_style.unfocused_selection_color =
                theme.color(&tokens::TEXT_INPUT_SELECTION_UNFOCUSED);
        }
    }
}

fn update_text_input_styles(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        (With<FeathersTextInputInner>, Added<InteractionDisabled>),
    >,
    mut commands: Commands,
) {
    for (input_ent, disabled, font_color) in q_inputs.iter() {
        set_text_input_styles(input_ent, disabled, font_color, &mut commands);
    }
}

fn update_text_input_styles_remove(
    q_inputs: Query<
        (Entity, Has<InteractionDisabled>, &InheritableThemeTextColor),
        With<FeathersTextInputInner>,
    >,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut commands: Commands,
) {
    removed_disabled.read().for_each(|ent| {
        if let Ok((input_ent, disabled, font_color)) = q_inputs.get(ent) {
            set_text_input_styles(input_ent, disabled, font_color, &mut commands);
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
                (
                    update_text_input_disabled_remove,
                    update_text_input_disabled,
                    update_text_input_value,
                    update_text_input_styles,
                    update_text_input_styles_remove,
                )
                    .chain(),
            )
                .in_set(PickingSystems::Last),
        );
    }
}
