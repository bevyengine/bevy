use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::event::EntityEvent;
use bevy_ecs::query::{Has, Without};
use bevy_ecs::system::{In, ResMut};
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

use crate::{Callback, Notify as _, ValueChange};

/// Headless widget implementation for checkboxes. The [`Checked`] component represents the current
/// state of the checkbox. The `on_change` field is an optional system id that will be run when the
/// checkbox is clicked, or when the `Enter` or `Space` key is pressed while the checkbox is
/// focused. If the `on_change` field is `Callback::Ignore`, then instead of calling a callback, the
/// checkbox will update its own [`Checked`] state directly.
///
/// # Toggle switches
///
/// The [`CoreCheckbox`] component can be used to implement other kinds of toggle widgets. If you
/// are going to do a toggle switch, you should override the [`AccessibilityNode`] component with
/// the `Switch` role instead of the `Checkbox` role.
#[derive(Component, Debug, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Checkable)]
pub struct CoreCheckbox {
    /// One-shot system that is run when the checkbox state needs to be changed. If this value is
    /// `Callback::Ignore`, then the checkbox will update it's own internal [`Checked`] state
    /// without notification.
    pub on_change: Callback<In<ValueChange<bool>>>,
}

fn checkbox_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_checkbox: Query<(&CoreCheckbox, Has<Checked>), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked)) = q_checkbox.get(ev.target()) {
        let event = &ev.event().input;
        if event.state == ButtonState::Pressed
            && !event.repeat
            && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
        {
            ev.propagate(false);
            set_checkbox_state(&mut commands, ev.target(), checkbox, !is_checked);
        }
    }
}

fn checkbox_on_pointer_click(
    mut ev: On<Pointer<Click>>,
    q_checkbox: Query<(&CoreCheckbox, Has<Checked>, Has<InteractionDisabled>)>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(ev.target()) {
        // Clicking on a button makes it the focused input,
        // and hides the focus ring if it was visible.
        if let Some(mut focus) = focus {
            focus.0 = Some(ev.target());
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = false;
        }

        ev.propagate(false);
        if !disabled {
            set_checkbox_state(&mut commands, ev.target(), checkbox, !is_checked);
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
/// use bevy_core_widgets::{CoreCheckbox, SetChecked};
///
/// fn setup(mut commands: Commands) {
///     // Create a checkbox
///     let checkbox = commands.spawn((
///         CoreCheckbox::default(),
///     )).id();
///
///     // Set to checked
///     commands.trigger_targets(SetChecked(true), checkbox);
/// }
/// ```
#[derive(EntityEvent)]
pub struct SetChecked(pub bool);

/// Event which can be triggered on a checkbox to toggle the checked state. This can be used to
/// control the checkbox via gamepad buttons or other inputs.
///
/// # Example:
///
/// ```
/// use bevy_ecs::system::Commands;
/// use bevy_core_widgets::{CoreCheckbox, ToggleChecked};
///
/// fn setup(mut commands: Commands) {
///     // Create a checkbox
///     let checkbox = commands.spawn((
///         CoreCheckbox::default(),
///     )).id();
///
///     // Set to checked
///     commands.trigger_targets(ToggleChecked, checkbox);
/// }
/// ```
#[derive(EntityEvent)]
pub struct ToggleChecked;

fn checkbox_on_set_checked(
    mut ev: On<SetChecked>,
    q_checkbox: Query<(&CoreCheckbox, Has<Checked>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(ev.target()) {
        ev.propagate(false);
        if disabled {
            return;
        }

        let will_be_checked = ev.event().0;
        if will_be_checked != is_checked {
            set_checkbox_state(&mut commands, ev.target(), checkbox, will_be_checked);
        }
    }
}

fn checkbox_on_toggle_checked(
    mut ev: On<ToggleChecked>,
    q_checkbox: Query<(&CoreCheckbox, Has<Checked>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(ev.target()) {
        ev.propagate(false);
        if disabled {
            return;
        }

        set_checkbox_state(&mut commands, ev.target(), checkbox, !is_checked);
    }
}

fn set_checkbox_state(
    commands: &mut Commands,
    entity: impl Into<bevy_ecs::entity::Entity>,
    checkbox: &CoreCheckbox,
    new_state: bool,
) {
    if !matches!(checkbox.on_change, Callback::Ignore) {
        commands.notify_with(
            &checkbox.on_change,
            ValueChange {
                source: entity.into(),
                value: new_state,
            },
        );
    } else if new_state {
        commands.entity(entity.into()).insert(Checked);
    } else {
        commands.entity(entity.into()).remove::<Checked>();
    }
}

/// Plugin that adds the observers for the [`CoreCheckbox`] widget.
pub struct CoreCheckboxPlugin;

impl Plugin for CoreCheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(checkbox_on_key_input)
            .add_observer(checkbox_on_pointer_click)
            .add_observer(checkbox_on_set_checked)
            .add_observer(checkbox_on_toggle_checked);
    }
}
