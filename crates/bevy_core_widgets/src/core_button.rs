use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::query::Has;
use bevy_ecs::system::In;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::On,
    query::With,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy_ui::{InteractionDisabled, Pressed};

use crate::{Activate, Callback, Notify};

/// Headless button widget. This widget maintains a "pressed" state, which is used to
/// indicate whether the button is currently being pressed by the user. It emits a `ButtonClicked`
/// event when the button is un-pressed.
#[derive(Component, Default, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Button)))]
pub struct CoreButton {
    /// Callback to invoke when the button is clicked, or when the `Enter` or `Space` key
    /// is pressed while the button is focused.
    pub on_activate: Callback<In<Activate>>,
}

fn button_on_key_event(
    mut event: On<FocusedInput<KeyboardInput>>,
    q_state: Query<(&CoreButton, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, disabled)) = q_state.get(event.entity())
        && !disabled
    {
        let input_event = &event.input;
        if !input_event.repeat
            && input_event.state == ButtonState::Pressed
            && (input_event.key_code == KeyCode::Enter || input_event.key_code == KeyCode::Space)
        {
            event.propagate(false);
            commands.notify_with(&bstate.on_activate, Activate(event.entity()));
        }
    }
}

fn button_on_pointer_click(
    mut event: On<Pointer<Click>>,
    mut q_state: Query<(&CoreButton, Has<Pressed>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, pressed, disabled)) = q_state.get_mut(event.entity()) {
        event.propagate(false);
        if pressed && !disabled {
            commands.notify_with(&bstate.on_activate, Activate(event.entity()));
        }
    }
}

fn button_on_pointer_down(
    mut event: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(event.entity()) {
        event.propagate(false);
        if !disabled && !pressed {
            commands.entity(button).insert(Pressed);
        }
    }
}

fn button_on_pointer_up(
    mut event: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(event.entity()) {
        event.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_drag_end(
    mut event: On<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(event.entity()) {
        event.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_cancel(
    mut event: On<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(event.entity()) {
        event.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

/// Plugin that adds the observers for the [`CoreButton`] widget.
pub struct CoreButtonPlugin;

impl Plugin for CoreButtonPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(button_on_key_event)
            .add_observer(button_on_pointer_down)
            .add_observer(button_on_pointer_up)
            .add_observer(button_on_pointer_click)
            .add_observer(button_on_pointer_drag_end)
            .add_observer(button_on_pointer_cancel);
    }
}
