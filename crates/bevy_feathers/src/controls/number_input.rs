use bevy_app::PropagateOver;
use bevy_asset::AssetServer;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::{ChildOf, Children},
    observer::On,
    query::With,
    relationship::Relationship,
    system::{Commands, Query, Res},
    template::template,
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{FocusLost, FocusedInput, InputFocus};
use bevy_log::warn;
use bevy_scene::prelude::*;
use bevy_text::{
    EditableText, EditableTextFilter, FontSource, FontWeight, TextEdit, TextEditChange, TextFont,
};
use bevy_ui::{px, widget::Text, AlignItems, AlignSelf, Display, JustifyContent, Node, UiRect};
use bevy_ui_widgets::ValueChange;

use crate::{
    constants::{fonts, size},
    controls::{text_input, text_input_container, TextInputProps},
    theme::{ThemeBackgroundColor, ThemeBorderColor, ThemeTextColor, ThemeToken},
    tokens,
};

/// Marker to indicate a number input widget with feathers styling.
#[derive(Component, Default, Clone)]
struct FeathersNumberInput;

/// Used to indicate what format of numbers we are editing. This primarily affects the type
/// of [`ValueChange`] event that is emitted.
#[derive(Component, Default, Clone, Copy)]
pub enum NumberFormat {
    /// A 32-bit float
    #[default]
    F32,
    /// A 64-bit float
    F64,
    /// A 32-bit integer
    I32,
    /// A 64-bit integer
    I64,
}

/// Parameters for the text input template, passed to [`number_input`] function.
pub struct NumberInputProps {
    /// The "sigil" is a colored strip along the left edge of the input, which is used to
    /// distinguish between different axes. The default is transparent (no sigil).
    pub sigil_color: ThemeToken,
    /// A caption to be placed on the left side of the input, next to the colored stripe.
    /// Usually one of "X", "Y" or "Z".
    pub label_text: Option<&'static str>,
    /// Indicate what size numbers we are editing.
    pub number_format: NumberFormat,
}

impl Default for NumberInputProps {
    fn default() -> Self {
        Self {
            sigil_color: tokens::TEXT_INPUT_BG,
            label_text: None,
            number_format: NumberFormat::F32,
        }
    }
}

/// Represents numbers in different formats.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum NumberInputValue {
    /// An f32 value
    F32(f32),
    /// An f64 value
    F64(f64),
    /// An i32 value
    I32(i32),
    /// An i64 value
    I64(i64),
}

impl core::fmt::Display for NumberInputValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NumberInputValue::F32(v) => write!(f, "{}", v),
            NumberInputValue::F64(v) => write!(f, "{}", v),
            NumberInputValue::I32(v) => write!(f, "{}", v),
            NumberInputValue::I64(v) => write!(f, "{}", v),
        }
    }
}

/// Event which can be sent to the number input widget to update the displayed value.
#[derive(Clone, EntityEvent)]
pub struct UpdateNumberInput {
    /// Target widget
    pub entity: Entity,

    /// Value to change to
    pub value: NumberInputValue,
}

/// Widget that permits text entry of floating-point numbers. This widget implements two-way
/// synchronization:
/// * when the widget has focus, it emits values (via a [`ValueChange<T>`]) event as the user types.
///   The type of ``T`` will be ``f32``, ``f64``, ``i32``, or ``i64`` depending on the
///   ``number_format`` parameter.
/// * when the widget does not have focus, it listens for [`UpdateNumberInput`] events, and replaces
///   the contents of the text buffer based on the value in that event.
///
/// To avoid excessive updating, you should only update the number value when there is an actual
/// change, that is, when the new value is different from the current value.
///
/// In most cases, the actual source of truth for the numeric value will be external, that is,
/// some property in an app-specific data structure. It's the responsibility of the app to
/// synchronize this value with the [`number_input`] widget in both directions:
/// * When a [`ValueChange`] event is received, update the app-specific property.
/// * When the app-specific property changes - either in response to a [`ValueChange`] event, or
///   because of some other action, trigger an [`UpdateNumberInput`] entity event to update the
///   displayed value.
// TODO: Add text_input field validation when it becomes available.
pub fn number_input(props: NumberInputProps) -> impl Scene {
    bsn! {
        :text_input_container()
        ThemeBorderColor({props.sigil_color.clone()})
        FeathersNumberInput
        template_value(props.number_format)
        on(number_input_on_update)
        Children [
            {
                match props.label_text {
                    Some(text) => Box::new(bsn_list!(
                        Node {
                            display: Display::Flex,
                            align_items: AlignItems::Center,
                            align_self: AlignSelf::Stretch,
                            justify_content: JustifyContent::Center,
                            padding: UiRect::axes(px(6), px(0)),
                        }
                        ThemeBackgroundColor(tokens::TEXT_INPUT_LABEL_BG)
                        Children [
                            Text::new(text.to_string())
                            template(|ctx| {
                                Ok(TextFont {
                                    font: FontSource::Handle(ctx.resource::<AssetServer>().load(fonts::REGULAR)),
                                    font_size: size::COMPACT_FONT,
                                    weight: FontWeight::NORMAL,
                                    ..Default::default()
                                })
                            })
                            PropagateOver<TextFont>
                            ThemeTextColor(tokens::TEXT_INPUT_TEXT)
                        ]
                    )) as Box<dyn SceneList>,
                    None => Box::new(bsn_list!()) as Box<dyn SceneList>
                }
            }
            text_input(TextInputProps {
                visible_width: None,
                max_characters: Some(20),
            })
            on(number_input_on_text_change)
            on(number_input_on_enter_key)
            on(number_input_on_focus_loss)
            EditableTextFilter::new(|c| {
                c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E')
            }),
        ]
    }
}

fn number_input_on_text_change(
    change: On<TextEditChange>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    q_text_input: Query<&EditableText>,
    mut commands: Commands,
) {
    let Ok(parent) = q_parent.get(change.event_target()) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get(change.event_target()) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands, false);
}

fn number_input_on_update(
    update: On<UpdateNumberInput>,
    q_children: Query<&Children>,
    q_number_input: Query<(), With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    focus: Res<InputFocus>,
) {
    if !q_number_input.contains(update.event_target()) {
        return;
    };

    let Ok(children) = q_children.get(update.event_target()) else {
        return;
    };

    for child_id in children.iter() {
        if focus.get() != Some(*child_id)
            && let Ok(mut editable_text) = q_text_input.get_mut(*child_id)
        {
            let new_digits = update.value.to_string();
            let old_digits = editable_text.value().to_string();
            if old_digits != new_digits {
                editable_text.queue_edit(TextEdit::SelectAll);
                editable_text.queue_edit(TextEdit::Insert(new_digits.into()));
            }
            break;
        }
    }
}

fn number_input_on_enter_key(
    key_input: On<FocusedInput<KeyboardInput>>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    q_text_input: Query<&EditableText>,
    mut commands: Commands,
) {
    if key_input.input.key_code != KeyCode::Enter {
        return;
    }

    let Ok(parent) = q_parent.get(key_input.event_target()) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get(key_input.event_target()) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands, true);
}

fn number_input_on_focus_loss(
    focus_lost: On<FocusLost>,
    q_parent: Query<&ChildOf>,
    q_number_input: Query<&NumberFormat, With<FeathersNumberInput>>,
    mut q_text_input: Query<&mut EditableText>,
    mut commands: Commands,
) {
    let editable_text_id = focus_lost.event_target();

    let Ok(parent) = q_parent.get(editable_text_id) else {
        return;
    };

    let Ok(number_format) = q_number_input.get(parent.get()) else {
        return;
    };

    let Ok(editable_text) = q_text_input.get_mut(editable_text_id) else {
        return;
    };

    let text_value = editable_text.value().to_string();
    emit_value_change(text_value, *number_format, parent.0, &mut commands, true);
}

fn emit_value_change(
    text_value: String,
    format: NumberFormat,
    source: Entity,
    commands: &mut Commands,
    is_final: bool,
) {
    let text_value = text_value.trim();
    if text_value.is_empty() {
        return;
    }

    match format {
        NumberFormat::F32 => {
            match text_value.parse::<f32>() {
                Ok(new_value) => {
                    commands.trigger(ValueChange {
                        source,
                        value: new_value,
                        is_final,
                    });
                }
                Err(_) => {
                    // TODO: Emit a validation error once these are defined
                    warn!("Invalid floating-point number in text edit");
                }
            }
        }
        NumberFormat::F64 => {
            match text_value.parse::<f64>() {
                Ok(new_value) => {
                    commands.trigger(ValueChange {
                        source,
                        value: new_value,
                        is_final,
                    });
                }
                Err(_) => {
                    // TODO: Emit a validation error once these are defined
                    warn!("Invalid floating-point number in text edit");
                }
            }
        }
        NumberFormat::I32 => {
            match text_value.parse::<i32>() {
                Ok(new_value) => {
                    commands.trigger(ValueChange {
                        source,
                        value: new_value,
                        is_final,
                    });
                }
                Err(_) => {
                    // TODO: Emit a validation error once these are defined
                    warn!("Invalid integer number in text edit");
                }
            }
        }
        NumberFormat::I64 => {
            match text_value.parse::<i64>() {
                Ok(new_value) => {
                    commands.trigger(ValueChange {
                        source,
                        value: new_value,
                        is_final,
                    });
                }
                Err(_) => {
                    // TODO: Emit a validation error once these are defined
                    warn!("Invalid integer number in text edit");
                }
            }
        }
    }
}
