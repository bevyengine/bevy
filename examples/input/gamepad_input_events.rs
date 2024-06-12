//! Iterates and prints gamepad input and connection events.

use bevy::{
    input::gamepad::{
        GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadButtonStateChangedEvent,
        GamepadConnectionEvent, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
        RawGamepadEvent,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, (gamepad_events, gamepad_ordered_events))
        .run();
}

fn gamepad_events(
    mut connection_events: EventReader<GamepadConnectionEvent>,
    // Handles the continuous measure of an axis, equivalent to GamepadAxes::get.
    // Whenever that *raw* value changes, this event is emitted.
    mut raw_axis_changed_events: EventReader<RawGamepadAxisChangedEvent>,
    // Same as above but filtered by the settings in `GamepadSettings`.
    mut axis_changed_events: EventReader<GamepadAxisChangedEvent>,
    // Handles the continuous measure of how far a button has been pressed down, equivalent to `GamepadButtons::get`.
    // Whenever that *raw* value changes, this event is emitted.
    mut raw_button_changed_events: EventReader<RawGamepadButtonChangedEvent>,
    // Same as above but filtered by the settings in `GamepadSettings`.
    mut button_changed_events: EventReader<GamepadButtonChangedEvent>,
    // Handles the boolean measure of whether a button is considered pressed or unpressed, as
    // defined by the thresholds in `GamepadSettings::button_settings`.
    // When the threshold is crossed and the button state changes, this event is emitted.
    mut button_input_events: EventReader<GamepadButtonStateChangedEvent>,
) {
    for connection_event in connection_events.read() {
        info!("{:?}", connection_event);
    }
    for axis_changed_event in raw_axis_changed_events.read() {
        info!(
            "Raw: {:?} of {:?} is changed to {}",
            axis_changed_event.axis, axis_changed_event.gamepad, axis_changed_event.value
        );
    }
    for axis_changed_event in axis_changed_events.read() {
        info!(
            "{:?} of {:?} is changed to {}",
            axis_changed_event.axis, axis_changed_event.gamepad, axis_changed_event.value
        );
    }
    for button_changed_event in raw_button_changed_events.read() {
        info!(
            "Raw: {:?} of {:?} is changed to {}",
            button_changed_event.button, button_changed_event.gamepad, button_changed_event.value
        );
    }
    for button_changed_event in button_changed_events.read() {
        info!(
            "{:?} of {:?} is changed to {}",
            button_changed_event.button, button_changed_event.gamepad, button_changed_event.value
        );
    }
    for button_input_event in button_input_events.read() {
        info!("{:?}", button_input_event);
    }
}

// If you require in-frame relative event ordering, you can also read the `Gamepad` event
// stream directly. For standard use-cases, reading the events individually or using the
// `Input<T>` or `Axis<T>` resources is preferable.
fn gamepad_ordered_events(mut gamepad_events: EventReader<RawGamepadEvent>) {
    for gamepad_event in gamepad_events.read() {
        match gamepad_event {
            RawGamepadEvent::Connection(connection_event) => info!("{:?}", connection_event),
            RawGamepadEvent::Button(button_event) => info!("{:?}", button_event),
            RawGamepadEvent::Axis(axis_event) => info!("{:?}", axis_event),
        }
    }
}
