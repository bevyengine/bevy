use bevy_ecs::prelude::*;
use bevy_picking::events::{Down, Out, Over, Pointer, Up};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The data required to create a buttonlike widget that can respond to hovering and being clicked / pressed.
///
/// Information about whether or not this button is currently pressed or hovered is gathered from
/// user input, processed via [`bevy_picking`] and read in the [`determine_button_interaction`] system.
///
/// See [`FocusPolicy`](crate::focus::FocusPolicy) to configure whether or not the button should block interactions with lower nodes.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Button {
    /// Whether the button is currently pressed.
    ///
    /// This is `true` as long as the button is pressed, but only while the pointer
    /// is over the button.
    pub pressed: bool,
    /// Whether the button is currently hovered.
    ///
    /// This is only `true` while the pointer is over this button,
    /// and will not be set if there is no pointer.
    pub hovered: bool,
}

/// An observer that watches for [`Pointer<Over>`] events and sets the [`Button::hovered`] field to `true`.
pub fn button_hover_observer(
    trigger: Trigger<Pointer<Over>>,
    mut button_query: Query<&mut Button>,
) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.hovered = true;
    }
}

/// An observer that watches for [`Pointer<Out>`] events and sets the [`Button::hovered`] field to `false`.
///
/// [`Button::pressed`] is also set to `false`, to ensure that the button.
pub fn button_out_observer(trigger: Trigger<Pointer<Out>>, mut button_query: Query<&mut Button>) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.hovered = false;
        button.pressed = false;
    }
}

/// An observer that watches for [`Pointer<Down>`] events and sets the [`Button::pressed`] field to `true`.
pub fn button_down_observer(trigger: Trigger<Pointer<Down>>, mut button_query: Query<&mut Button>) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.pressed = true;
    }
}

/// An observer that watches for [`Pointer<Up>`] events and sets the [`Button::pressed`] field to `false`.
pub fn button_up_observer(trigger: Trigger<Pointer<Up>>, mut button_query: Query<&mut Button>) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.pressed = false;
    }
}
