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
use bevy_ecs::entity::Entity;

/// Headless widget implementation for checkboxes. The [`Checked`] component represents the current
/// state of the checkbox. The `on_change` field is an optional system id that will be run when the
/// checkbox is clicked, or when the `Enter` or `Space` key is pressed while the checkbox is
/// focused. If the `on_change` field is `Callback::Ignore`, then instead of calling a callback, the
/// checkbox will update its own [`Checked`] state directly.
///
/// # Toggle switches
///
/// The [`Checkbox`] component can be used to implement other kinds of toggle widgets. If you
/// are going to do a toggle switch, you should override the [`AccessibilityNode`] component with
/// the `Switch` role instead of the `Checkbox` role.
#[derive(Component, Debug, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Checkable)]
pub struct Checkbox {
    /// One-shot system that is run when the checkbox state needs to be changed. If this value is
    /// `Callback::Ignore`, then the checkbox will update it's own internal [`Checked`] state
    /// without notification.
    pub on_change: Callback<In<ValueChange<bool>>>,
}

fn checkbox_on_key_input(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_checkbox: Query<(&Checkbox, Has<Checked>), Without<InteractionDisabled>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked)) = q_checkbox.get(ev.focused_entity) {
        let event = &ev.event().input;
        if event.state == ButtonState::Pressed
            && !event.repeat
            && (event.key_code == KeyCode::Enter || event.key_code == KeyCode::Space)
        {
            ev.propagate(false);
            set_checkbox_state(&mut commands, ev.focused_entity, checkbox, !is_checked);
        }
    }
}

fn checkbox_on_pointer_click(
    mut click: On<Pointer<Click>>,
    q_checkbox: Query<(&Checkbox, Has<Checked>, Has<InteractionDisabled>)>,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(click.entity) {
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
            set_checkbox_state(&mut commands, click.entity, checkbox, !is_checked);
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
    q_checkbox: Query<(&Checkbox, Has<Checked>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(set_checked.entity) {
        if disabled {
            return;
        }

        let will_be_checked = set_checked.checked;
        if will_be_checked != is_checked {
            set_checkbox_state(&mut commands, set_checked.entity, checkbox, will_be_checked);
        }
    }
}

fn checkbox_on_toggle_checked(
    toggle_checked: On<ToggleChecked>,
    q_checkbox: Query<(&Checkbox, Has<Checked>, Has<InteractionDisabled>)>,
    mut commands: Commands,
) {
    if let Ok((checkbox, is_checked, disabled)) = q_checkbox.get(toggle_checked.entity) {
        if disabled {
            return;
        }

        set_checkbox_state(&mut commands, toggle_checked.entity, checkbox, !is_checked);
    }
}

fn set_checkbox_state(
    commands: &mut Commands,
    entity: impl Into<Entity>,
    checkbox: &Checkbox,
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
