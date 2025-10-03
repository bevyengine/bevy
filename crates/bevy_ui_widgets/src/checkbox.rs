use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::event::EntityEvent;
use bevy_ecs::query::{Has, With, Without};
use bevy_ecs::system::ResMut;
use bevy_ecs::{
    component::Component,
    observer::On,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::{FocusedInput, InputFocus, InputFocusVisible};
use bevy_picking::events::{Click, Pointer};
use bevy_ui::{Checkable, Checked, InteractionDisabled};

use crate::ValueChange;
use bevy_ecs::entity::Entity;

/// Headless widget implementation for checkboxes. The [`Checked`] component represents the current
/// state of the checkbox. The widget will emit a [`ValueChange<bool>`] event when clicked, or when
/// the `Enter` or `Space` key is pressed while the checkbox is focused.
///
/// Add the [`checkbox_self_update`] observer watching the entity with this component to automatically add and remove the [`Checked`] component.
///
/// # Toggle switches
///
/// The [`Checkbox`] component can be used to implement other kinds of toggle widgets. If you
/// are going to do a toggle switch, you should override the [`AccessibilityNode`] component with
/// the `Switch` role instead of the `Checkbox` role.
#[derive(Component, Debug, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Checkable)]
pub struct Checkbox;

fn checkbox_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_checkbox: Query<Has<Checked>, (With<Checkbox>, Without<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok(is_checked) = q_checkbox.get(ev.focused_entity) {
        let event = &ev.event().input;
        if event.state == ButtonState::Pressed
            && !event.repeat
            && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
        {
            ev.propagate(false);
            commands.trigger(ValueChange {
                source: ev.focused_entity,
                value: !is_checked,
            });
        }
    }
}

fn checkbox_on_pointer_click(
    mut click: On<Pointer<Click>>,
    q_checkbox: Query<(Has<Checked>, Has<InteractionDisabled>), With<Checkbox>>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if let Ok((is_checked, disabled)) = q_checkbox.get(click.entity) {
        // Clicking on a button makes it the focused input,
        // and hides the focus ring if it was visible.
        if let Some(mut focus) = focus {
            focus.0 = Some(click.entity);
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = false;
        }

        click.propagate(false);
        if !disabled {
            commands.trigger(ValueChange {
                source: click.entity,
                value: !is_checked,
            });
        }
    }
}

/// Event which can be triggered on a checkbox to set the checked state. This can be used to control
/// the checkbox via gamepad buttons or other inputs.
///
/// # Example:
///
/// ```
/// use bevy_ecs::system::Commands;
/// use bevy_ui_widgets::{Checkbox, SetChecked};
///
/// fn setup(mut commands: Commands) {
///     // Create a checkbox
///     let entity = commands.spawn((
///         Checkbox::default(),
///     )).id();
///
///     // Set to checked
///     commands.trigger(SetChecked { entity, checked: true});
/// }
/// ```
#[derive(EntityEvent)]
pub struct SetChecked {
    /// The [`Checkbox`] entity to set the "checked" state on.
    pub entity: Entity,
    /// Sets the `checked` state to `true` or `false`.
    pub checked: bool,
}

/// Event which can be triggered on a checkbox to toggle the checked state. This can be used to
/// control the checkbox via gamepad buttons or other inputs.
///
/// # Example:
///
/// ```
/// use bevy_ecs::system::Commands;
/// use bevy_ui_widgets::{Checkbox, ToggleChecked};
///
/// fn setup(mut commands: Commands) {
///     // Create a checkbox
///     let entity = commands.spawn((
///         Checkbox::default(),
///     )).id();
///
///     // Set to checked
///     commands.trigger(ToggleChecked { entity });
/// }
/// ```
#[derive(EntityEvent)]
pub struct ToggleChecked {
    /// The [`Entity`] of the toggled [`Checkbox`]
    pub entity: Entity,
}

fn checkbox_on_set_checked(
    set_checked: On<SetChecked>,
    q_checkbox: Query<(Has<Checked>, Has<InteractionDisabled>), With<Checkbox>>,
    mut commands: Commands,
) {
    if let Ok((is_checked, disabled)) = q_checkbox.get(set_checked.entity) {
        if disabled {
            return;
        }

        let will_be_checked = set_checked.checked;
        if will_be_checked != is_checked {
            commands.trigger(ValueChange {
                source: set_checked.entity,
                value: will_be_checked,
            });
        }
    }
}

fn checkbox_on_toggle_checked(
    toggle_checked: On<ToggleChecked>,
    q_checkbox: Query<(Has<Checked>, Has<InteractionDisabled>), With<Checkbox>>,
    mut commands: Commands,
) {
    if let Ok((is_checked, disabled)) = q_checkbox.get(toggle_checked.entity) {
        if disabled {
            return;
        }

        commands.trigger(ValueChange {
            source: toggle_checked.entity,
            value: !is_checked,
        });
    }
}

/// Plugin that adds the observers for the [`Checkbox`] widget.
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(checkbox_on_key_input)
            .add_observer(checkbox_on_pointer_click)
            .add_observer(checkbox_on_set_checked)
            .add_observer(checkbox_on_toggle_checked);
    }
}

/// Observer function which updates the checkbox value in response to a [`ValueChange`] event.
/// This can be used to make the checkbox automatically update its own state when clicked,
/// as opposed to managing the checkbox state externally.
pub fn checkbox_self_update(value_change: On<ValueChange<bool>>, mut commands: Commands) {
    if value_change.value {
        commands.entity(value_change.source).insert(Checked);
    } else {
        commands.entity(value_change.source).remove::<Checked>();
    }
}
