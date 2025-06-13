use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::query::Has;
use bevy_ecs::system::ResMut;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::On,
    query::With,
    system::{Commands, Query, SystemId},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy_ui::{InteractionDisabled, Pressed};

/// Headless button widget. This widget maintains a "pressed" state, which is used to
/// indicate whether the button is currently being pressed by the user. It emits a `ButtonClicked`
/// event when the button is un-pressed.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Button)))]
pub struct CoreButton {
    /// Optional system to run when the button is clicked, or when the Enter or Space key
    /// is pressed while the button is focused. If this field is `None`, the button will
    /// emit a `ButtonClicked` event when clicked.
    pub on_click: Option<SystemId>,
}

fn button_on_key_event(
    mut trigger: On<FocusedInput<KeyboardInput>>,
    q_state: Query<(&CoreButton, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, disabled)) = q_state.get(trigger.target().unwrap()) {
        if !disabled {
            let event = &trigger.event().input;
            if !event.repeat
                && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
            {
                if let Some(on_click) = bstate.on_click {
                    trigger.propagate(false);
                    commands.run_system(on_click);
                }
            }
        }
    }
}

fn button_on_pointer_click(
    mut trigger: On<Pointer<Click>>,
    mut q_state: Query<(&CoreButton, Has<Pressed>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((bstate, pressed, disabled)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if pressed && !disabled {
            if let Some(on_click) = bstate.on_click {
                commands.run_system(on_click);
            }
        }
    }
}

fn button_on_pointer_down(
    mut trigger: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if !disabled {
            if !pressed {
                commands.entity(button).insert(Pressed);
            }
            // Clicking on a button makes it the focused input,
            // and hides the focus ring if it was visible.
            if let Some(mut focus) = focus {
                focus.0 = trigger.target();
            }
            if let Some(mut focus_visible) = focus_visible {
                focus_visible.0 = false;
            }
        }
    }
}

fn button_on_pointer_up(
    mut trigger: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_drag_end(
    mut trigger: On<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_cancel(
    mut trigger: On<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<CoreButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(trigger.target().unwrap()) {
        trigger.propagate(false);
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
