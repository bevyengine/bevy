use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::event::EntityEvent;
use bevy_ecs::query::{Has, With, Without};
use bevy_ecs::system::ResMut;
use bevy_ecs::{
    component::Component,
    observer::On,
    reflect::{ReflectComponent, ReflectEvent},
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy_reflect::Reflect;
use bevy_ui::{Checkable, Checked, InteractionDisabled, Pressed};

use crate::{ActivateOnPress, ValueChange};
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
#[derive(Component, Debug, Default, Clone)]
#[require(AccessibilityNode(accesskit::Node::new(Role::CheckBox)), Checkable)]
#[derive(Reflect)]
#[reflect(Component)]
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
                is_final: true,
            });
        }
    }
}

fn checkbox_on_pointer_click(
    mut click: On<Pointer<Click>>,
    q_checkbox: Query<
        (Has<Checked>, Has<InteractionDisabled>),
        (With<Checkbox>, Without<ActivateOnPress>),
    >,
    mut commands: Commands,
) {
    if let Ok((is_checked, disabled)) = q_checkbox.get(click.entity) {
        click.propagate(false);
        if !disabled {
            commands.trigger(ValueChange {
                source: click.entity,
                value: !is_checked,
                is_final: true,
            });
        }
    }
}

fn checkbox_on_pointer_down(
    mut press: On<Pointer<Press>>,
    mut q_checkbox: Query<
        (
            Entity,
            Has<InteractionDisabled>,
            Has<Checked>,
            Has<Pressed>,
            Has<ActivateOnPress>,
        ),
        With<Checkbox>,
    >,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, disabled, checked, pressed, activate_on_press)) =
        q_checkbox.get_mut(press.entity)
    {
        // Clicking on a button makes it the focused input,
        // and hides the focus ring if it was visible.
        if let Some(mut focus) = focus {
            focus.set(press.entity, FocusCause::Pressed);
        }
        if let Some(mut focus_visible) = focus_visible {
            focus_visible.0 = false;
        }

        press.propagate(false);
        if !disabled && !pressed {
            commands.entity(checkbox).insert(Pressed);
            if activate_on_press {
                commands.trigger(ValueChange {
                    source: press.entity,
                    value: !checked,
                    is_final: true,
                });
            }
        }
    }
}

fn checkbox_on_pointer_up(
    mut release: On<Pointer<Release>>,
    mut q_checkbox: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Checkbox>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, disabled, pressed)) = q_checkbox.get_mut(release.entity) {
        release.propagate(false);
        if !disabled && pressed {
            commands.entity(checkbox).remove::<Pressed>();
        }
    }
}

fn checkbox_on_pointer_drag_end(
    mut drag_end: On<Pointer<DragEnd>>,
    mut q_checkbox: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Checkbox>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, disabled, pressed)) = q_checkbox.get_mut(drag_end.entity) {
        drag_end.propagate(false);
        if !disabled && pressed {
            commands.entity(checkbox).remove::<Pressed>();
        }
    }
}

fn checkbox_on_pointer_cancel(
    mut cancel: On<Pointer<Cancel>>,
    mut q_checkbox: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<Checkbox>>,
    mut commands: Commands,
) {
    if let Ok((checkbox, disabled, pressed)) = q_checkbox.get_mut(cancel.entity) {
        cancel.propagate(false);
        if !disabled && pressed {
            commands.entity(checkbox).remove::<Pressed>();
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
#[derive(EntityEvent, Reflect)]
#[reflect(Event)]
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
#[derive(EntityEvent, Reflect)]
#[reflect(Event)]
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
                is_final: true,
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
            is_final: true,
        });
    }
}

/// Plugin that adds the observers for the [`Checkbox`] widget.
pub struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(checkbox_on_key_input)
            .add_observer(checkbox_on_pointer_click)
            .add_observer(checkbox_on_pointer_down)
            .add_observer(checkbox_on_pointer_up)
            .add_observer(checkbox_on_pointer_drag_end)
            .add_observer(checkbox_on_pointer_cancel)
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::hierarchy::ChildOf;
    use bevy_input::keyboard::Key;
    use bevy_input::InputPlugin;
    use bevy_input_focus::{
        tab_navigation::{TabIndex, TabNavigationPlugin},
        InputDispatchPlugin, InputFocusPlugin,
    };
    use bevy_math::Vec2;
    use bevy_picking::backend::HitData;
    use bevy_picking::pointer::{Location, PointerId};
    use bevy_window::{PrimaryWindow, Window, WindowRef};

    /// Builds a headless app wired the way the `standard_widgets` example is: focus plugins plus the
    /// checkbox observers, with the [`checkbox_self_update`] observer so that a `ValueChange` is
    /// reflected back into the [`Checked`] component (mirroring how the example drives it).
    ///
    /// [`InputDispatchPlugin`] is included so that raw [`KeyboardInput`] messages are dispatched to
    /// the focused entity as `FocusedInput<KeyboardInput>` events (which is how `checkbox_on_key_input`
    /// receives them).
    fn checkbox_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins((
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            TabNavigationPlugin,
            CheckboxPlugin,
        ));
        app.add_observer(checkbox_self_update);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        // Resolve initial focus (goes to the primary window).
        app.update();
        (app, window)
    }

    /// A [`Location`] pointing at the primary window; the exact position is irrelevant because these
    /// tests drive the widget observers directly rather than through hit-testing.
    fn window_location(window: Entity) -> Location {
        Location {
            target: bevy_camera::NormalizedRenderTarget::Window(
                WindowRef::Entity(window).normalize(Some(window)).unwrap(),
            ),
            position: Vec2::ZERO,
        }
    }

    fn stub_hit(window: Entity) -> HitData {
        HitData::new(window, 0.0, None, None)
    }

    /// Synthesizes the pointer events a real click produces (`Press` → `Release` → `Click`) on
    /// `target`, mirroring what the picking backend would emit. `target` may be a non-focusable
    /// descendant of the widget; the events bubble up via `ChildOf` just like real pointer events.
    fn click_entity(app: &mut App, target: Entity, window: Entity) {
        let location = window_location(window);
        let button = bevy_picking::pointer::PointerButton::Primary;
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            location.clone(),
            Press {
                button,
                hit: stub_hit(window),
                count: 1,
            },
            target,
        ));
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            location.clone(),
            Release {
                button,
                hit: stub_hit(window),
            },
            target,
        ));
        app.world_mut().trigger(Pointer::new(
            PointerId::Mouse,
            location,
            Click {
                button,
                hit: stub_hit(window),
                duration: core::time::Duration::from_millis(10),
                count: 1,
            },
            target,
        ));
        app.update();
    }

    /// Clicking a checkbox toggles it and gives it focus.
    #[test]
    fn click_toggles_and_focuses_checkbox() {
        let (mut app, window) = checkbox_app();
        let checkbox = app
            .world_mut()
            .spawn((Checkbox, TabIndex(0), ChildOf(window)))
            .id();
        app.update();

        assert!(!app.world().entity(checkbox).contains::<Checked>());

        click_entity(&mut app, checkbox, window);

        assert!(
            app.world().entity(checkbox).contains::<Checked>(),
            "checkbox should toggle to checked on click"
        );
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(checkbox),
            "checkbox should receive focus on click"
        );

        // A second click toggles it back off.
        click_entity(&mut app, checkbox, window);
        assert!(!app.world().entity(checkbox).contains::<Checked>());
    }

    /// Clicking on a non-focusable child of the checkbox still toggles and
    /// focuses the checkbox root, because both the pointer events and the resulting `AcquireFocus` bubble up to it.
    #[test]
    fn click_on_child_toggles_and_focuses_checkbox_root() {
        let (mut app, window) = checkbox_app();
        let checkbox = app
            .world_mut()
            .spawn((Checkbox, TabIndex(0), ChildOf(window)))
            .id();
        let outer = app.world_mut().spawn(ChildOf(checkbox)).id();
        let inner = app.world_mut().spawn(ChildOf(outer)).id();
        app.update();

        click_entity(&mut app, inner, window);

        assert!(
            app.world().entity(checkbox).contains::<Checked>(),
            "clicking the inner node should toggle the checkbox root"
        );
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(checkbox),
            "clicking the inner node should focus the checkbox root"
        );
    }

    /// With a checkbox focused, pressing Space toggles it.
    #[test]
    fn space_key_toggles_focused_checkbox() {
        let (mut app, window) = checkbox_app();
        let checkbox = app
            .world_mut()
            .spawn((Checkbox, TabIndex(0), ChildOf(window)))
            .id();
        app.update();

        // Focus the checkbox as Tab navigation would.
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(checkbox, FocusCause::Navigated);

        // Send a raw Space key press; `InputDispatchPlugin` routes it to the focused entity as a
        // `FocusedInput<KeyboardInput>` event, which `checkbox_on_key_input` handles.
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::Space,
            logical_key: Key::Space,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();

        assert!(
            app.world().entity(checkbox).contains::<Checked>(),
            "Space should toggle the focused checkbox"
        );
    }

    /// A disabled checkbox does not toggle on click.
    #[test]
    fn disabled_checkbox_does_not_toggle() {
        let (mut app, window) = checkbox_app();
        let checkbox = app
            .world_mut()
            .spawn((Checkbox, InteractionDisabled, TabIndex(0), ChildOf(window)))
            .id();
        app.update();

        click_entity(&mut app, checkbox, window);

        assert!(
            !app.world().entity(checkbox).contains::<Checked>(),
            "a disabled checkbox must not toggle"
        );
    }
}
