use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::query::Has;
use bevy_ecs::resource::Resource;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::On,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy_ui::{InteractionDisabled, Pressed};

use crate::Activate;

/// Headless button widget. This widget maintains a "pressed" state, which is used to
/// indicate whether the button is currently being pressed by the user. It emits an [`Activate`]
/// event when the button is un-pressed.
#[derive(Component, Default, Debug, Clone)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Button)))]
pub struct Button;

/// A resource that holds the list of key codes that trigger an `Activate` event when pressed.
///
/// By default, it includes [`KeyCode::Enter`] and [`KeyCode::Space`].
#[derive(Resource)]
pub struct ButtonKeyEventCodes(Vec<KeyCode>);

impl ButtonKeyEventCodes {
    const DEFAULT_KEY_CODES: [KeyCode; 2] = [KeyCode::Enter, KeyCode::Space];
}

impl Default for ButtonKeyEventCodes {
    fn default() -> Self {
        Self(Self::DEFAULT_KEY_CODES.to_vec())
    }
}

fn button_on_key_event(
    mut event: On<FocusedInput<KeyboardInput>>,
    q_state: Query<Has<InteractionDisabled>, With<Button>>,
    mut commands: Commands,
    maybe_key_codes: Option<Res<ButtonKeyEventCodes>>,
) {
    if let Ok(disabled) = q_state.get(event.focused_entity)
        && !disabled
    {
        let input_event = &event.input;
        let is_valid_key_code = match maybe_key_codes {
            Some(key_codes) => key_codes.0.contains(&input_event.key_code),
            None => ButtonKeyEventCodes::DEFAULT_KEY_CODES.contains(&input_event.key_code),
        };
        if !input_event.repeat && input_event.state == ButtonState::Pressed && is_valid_key_code {
            event.propagate(false);
            commands.trigger(Activate {
                entity: event.focused_entity,
            });
        }
    }
}

fn button_on_pointer_click(
    mut click: On<Pointer<Click>>,
    mut q_state: Query<(Has<Pressed>, Has<InteractionDisabled>), With<Button>>,
    mut commands: Commands,
) {
    if let Ok((pressed, disabled)) = q_state.get_mut(click.entity) {
        click.propagate(false);
        if pressed && !disabled {
            commands.trigger(Activate {
                entity: click.entity,
            });
        }
    }
}

fn button_on_pointer_down(
    mut press: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Button>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(press.entity) {
        press.propagate(false);
        if !disabled && !pressed {
            commands.entity(button).insert(Pressed);
        }
    }
}

fn button_on_pointer_up(
    mut release: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Button>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(release.entity) {
        release.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Button>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(drag_end.entity) {
        drag_end.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

fn button_on_pointer_cancel(
    mut cancel: On<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Button>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(cancel.entity) {
        cancel.propagate(false);
        if !disabled && pressed {
            commands.entity(button).remove::<Pressed>();
        }
    }
}

/// Plugin that adds the observers for the [`Button`] widget.
pub struct ButtonPlugin;

impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ButtonKeyEventCodes>();

        app.add_observer(button_on_key_event)
            .add_observer(button_on_pointer_down)
            .add_observer(button_on_pointer_up)
            .add_observer(button_on_pointer_click)
            .add_observer(button_on_pointer_drag_end)
            .add_observer(button_on_pointer_cancel);
    }
}
