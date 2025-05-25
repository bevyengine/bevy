use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
#[cfg(feature = "bevy_input_focus")]
use bevy_ecs::system::ResMut;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::{Has, With},
    system::{Commands, Query, SystemId},
};
#[cfg(feature = "bevy_input_focus")]
use bevy_input::keyboard::{KeyCode, KeyboardInput};
#[cfg(feature = "bevy_input_focus")]
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Pressed, Released};
use bevy_ui::{ButtonPressed, InteractionDisabled};

use crate::events::ButtonClicked;

/// Headless button widget. This widget maintains a "pressed" state, which is used to
/// indicate whether the button is currently being pressed by the user. It emits a `ButtonClicked`
/// event when the button is un-pressed.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Button)))]
#[require(ButtonPressed)]
pub struct CoreButton {
    /// Optional system to run when the button is clicked, or when the Enter or Space key
    /// is pressed while the button is focused. If this field is `None`, the button will
    /// emit a `ButtonClicked` event when clicked.
    pub on_click: Option<SystemId>,
}

#[cfg(feature = "bevy_input_focus")]
fn button_on_key_event(
    mut trigger: Trigger<FocusedInput<KeyboardInput>>,
    q_state: Query<(&CoreButton, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, disabled)) = q_state.get(trigger.target()) {
        if !disabled {
            let event = &trigger.event().input;
            if !event.repeat
                && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
            {
                if let Some(on_click) = bstate.on_click {
                    trigger.propagate(false);
                    commands.run_system(on_click);
                } else {
                    commands.trigger_targets(ButtonClicked, trigger.target());
                }
            }
        }
    }
}

fn button_on_pointer_click(
    mut trigger: Trigger<Pointer<Click>>,
    mut q_state: Query<(&CoreButton, &ButtonPressed, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, pressed, disabled)) = q_state.get_mut(trigger.target()) {
        trigger.propagate(false);
        if pressed.0 && !disabled {
            if let Some(on_click) = bstate.on_click {
                commands.run_system(on_click);
            } else {
                commands.trigger_targets(ButtonClicked, trigger.target());
            }
        }
    }
}

fn button_on_pointer_down(
    mut trigger: Trigger<Pointer<Pressed>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
    #[cfg(feature = "bevy_input_focus")] mut focus: ResMut<InputFocus>,
    #[cfg(feature = "bevy_input_focus")] mut focus_visible: ResMut<InputFocusVisible>,
    mut commands: Commands,
) {
    if let Ok((button, disabled)) = q_state.get_mut(trigger.target()) {
        trigger.propagate(false);
        if !disabled {
            commands.entity(button).insert(ButtonPressed(true));
            // Clicking on a button makes it the focused input,
            // and hides the focus ring if it was visible.
            // #[cfg(feature = "bevy_input_focus")]
            focus.0 = Some(trigger.target());
            // #[cfg(feature = "bevy_input_focus")]
            focus_visible.0 = false;
        }
    }
}

fn button_on_pointer_up(
    mut trigger: Trigger<Pointer<Released>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled)) = q_state.get_mut(trigger.target()) {
        trigger.propagate(false);
        if !disabled {
            commands.entity(button).insert(ButtonPressed(false));
        }
    }
}

fn button_on_pointer_drag_end(
    mut trigger: Trigger<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled)) = q_state.get_mut(trigger.target()) {
        trigger.propagate(false);
        if !disabled {
            commands.entity(button).insert(ButtonPressed(false));
        }
    }
}

fn button_on_pointer_cancel(
    mut trigger: Trigger<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled)) = q_state.get_mut(trigger.target()) {
        trigger.propagate(false);
        if !disabled {
            commands.entity(button).insert(ButtonPressed(false));
        }
    }
}

/// Plugin that adds the observers for the `CoreButton` widget.
pub struct CoreButtonPlugin;

impl Plugin for CoreButtonPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_input_focus")]
        app.add_observer(button_on_key_event);
        app.add_observer(button_on_pointer_down)
            .add_observer(button_on_pointer_up)
            .add_observer(button_on_pointer_click)
            .add_observer(button_on_pointer_drag_end)
            .add_observer(button_on_pointer_cancel);
    }
}
