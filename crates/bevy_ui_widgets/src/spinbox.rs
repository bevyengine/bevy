use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    observer::On,
    query::With,
    relationship::Relationship,
    system::{Commands, Query},
};
use bevy_input::keyboard::{KeyCode, KeyboardInput};
use bevy_input::ButtonState;
use bevy_input_focus::FocusedInput;

use crate::{Activate, Button};

/// Headless spinbox container.
///
/// A spinbox composes increment and decrement buttons and emits direction intent from the spinbox
/// root. It does not assume a particular value type or editing surface.
///
/// ```ignore
/// use bevy_ecs::prelude::*;
/// use bevy_ui_widgets::{
///     SpinBox, SpinBoxButtonPress, SpinBoxDecrementButton, SpinBoxDirection,
///     SpinBoxIncrementButton,
/// };
///
/// let spinbox = commands.spawn(SpinBox).id();
/// commands.spawn((SpinBoxDecrementButton, ChildOf(spinbox)));
/// commands.spawn((SpinBoxIncrementButton, ChildOf(spinbox)));
///
/// commands.entity(spinbox).observe(|press: On<SpinBoxButtonPress>| match press.direction {
///     SpinBoxDirection::Increment => info!("next"),
///     SpinBoxDirection::Decrement => info!("previous"),
/// });
/// ```
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinBox;

/// The direction requested by a spinbox button activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinBoxDirection {
    /// Move to the next value.
    Increment,
    /// Move to the previous value.
    Decrement,
}

/// Marks the increment button of a [`SpinBox`].
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(Button)]
pub struct SpinBoxIncrementButton;

/// Marks the decrement button of a [`SpinBox`].
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(Button)]
pub struct SpinBoxDecrementButton;

/// Emitted when one of a spinbox's buttons is activated.
///
/// This always targets the [`SpinBox`] root entity, so apps can use it for arbitrary value
/// domains such as enums or wrap it with more specialized adapters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, EntityEvent)]
pub struct SpinBoxButtonPress {
    /// The spinbox entity.
    #[event_target]
    pub entity: Entity,
    /// The requested direction.
    pub direction: SpinBoxDirection,
}

pub(crate) fn spinbox_direction_for_key(key_code: KeyCode) -> Option<SpinBoxDirection> {
    match key_code {
        KeyCode::ArrowUp | KeyCode::ArrowRight => Some(SpinBoxDirection::Increment),
        KeyCode::ArrowDown | KeyCode::ArrowLeft => Some(SpinBoxDirection::Decrement),
        _ => None,
    }
}

fn spinbox_on_activate(
    activate: On<Activate>,
    q_increment: Query<(), With<SpinBoxIncrementButton>>,
    q_decrement: Query<(), With<SpinBoxDecrementButton>>,
    q_parent: Query<&ChildOf>,
    q_spinbox: Query<(), With<SpinBox>>,
    mut commands: Commands,
) {
    let button = activate.event_target();
    let direction = if q_increment.contains(button) {
        SpinBoxDirection::Increment
    } else if q_decrement.contains(button) {
        SpinBoxDirection::Decrement
    } else {
        return;
    };

    let Some(spinbox) = find_spinbox_ancestor(button, &q_parent, &q_spinbox) else {
        return;
    };

    commands.trigger(SpinBoxButtonPress {
        entity: spinbox,
        direction,
    });
}

fn spinbox_on_key_input(
    mut key_input: On<FocusedInput<KeyboardInput>>,
    q_spinbox: Query<(), With<SpinBox>>,
    q_parent: Query<&ChildOf>,
    mut commands: Commands,
) {
    let input_event = &key_input.input;
    if input_event.state != ButtonState::Pressed || input_event.repeat {
        return;
    }

    let Some(direction) = spinbox_direction_for_key(input_event.key_code) else {
        return;
    };

    let spinbox = if q_spinbox.contains(key_input.focused_entity) {
        Some(key_input.focused_entity)
    } else {
        find_spinbox_ancestor(key_input.focused_entity, &q_parent, &q_spinbox)
    };

    let Some(spinbox) = spinbox else {
        return;
    };

    key_input.propagate(false);
    commands.trigger(SpinBoxButtonPress {
        entity: spinbox,
        direction,
    });
}

pub(crate) fn find_spinbox_ancestor(
    entity: Entity,
    q_parent: &Query<&ChildOf>,
    q_spinbox: &Query<(), With<SpinBox>>,
) -> Option<Entity> {
    let mut current = entity;
    while let Ok(parent) = q_parent.get(current) {
        let parent = parent.get();
        if q_spinbox.contains(parent) {
            return Some(parent);
        }
        current = parent;
    }
    None
}

/// Plugin that adds observers for [`SpinBox`].
pub struct SpinBoxPlugin;

impl Plugin for SpinBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(spinbox_on_activate)
            .add_observer(spinbox_on_key_input);
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::{observer::On, prelude::*};
    use bevy_input::{
        keyboard::{Key, KeyboardInput},
        ButtonState, InputPlugin,
    };
    use bevy_input_focus::{InputDispatchPlugin, InputFocus, InputFocusPlugin};
    use bevy_window::{PrimaryWindow, Window};

    use super::*;

    #[derive(Resource, Default)]
    struct SpinBoxDirections(Vec<(Entity, SpinBoxDirection)>);

    fn keyboard_input(key_code: KeyCode) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key: match key_code {
                KeyCode::ArrowUp => Key::ArrowUp,
                KeyCode::ArrowDown => Key::ArrowDown,
                KeyCode::ArrowLeft => Key::ArrowLeft,
                KeyCode::ArrowRight => Key::ArrowRight,
                _ => unreachable!(),
            },
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        }
    }

    #[test]
    fn spinbox_emits_increment_button_press_from_root() {
        let mut app = App::new();
        app.init_resource::<SpinBoxDirections>()
            .add_plugins((crate::ButtonPlugin, SpinBoxPlugin))
            .add_observer(
                |press: On<SpinBoxButtonPress>, mut directions: ResMut<SpinBoxDirections>| {
                    directions.0.push((press.entity, press.direction));
                },
            );

        let spinbox = app.world_mut().spawn(SpinBox).id();
        let increment = app
            .world_mut()
            .spawn((SpinBoxIncrementButton, ChildOf(spinbox)))
            .id();

        app.world_mut()
            .commands()
            .trigger(Activate { entity: increment });
        app.update();

        assert_eq!(
            app.world().resource::<SpinBoxDirections>().0,
            vec![(spinbox, SpinBoxDirection::Increment)]
        );
    }

    #[test]
    fn spinbox_emits_decrement_button_press_without_value_input() {
        let mut app = App::new();
        app.init_resource::<SpinBoxDirections>()
            .add_plugins((crate::ButtonPlugin, SpinBoxPlugin))
            .add_observer(
                |press: On<SpinBoxButtonPress>, mut directions: ResMut<SpinBoxDirections>| {
                    directions.0.push((press.entity, press.direction));
                },
            );

        let spinbox = app.world_mut().spawn(SpinBox).id();
        let decrement = app
            .world_mut()
            .spawn((SpinBoxDecrementButton, ChildOf(spinbox)))
            .id();

        app.world_mut()
            .commands()
            .trigger(Activate { entity: decrement });
        app.update();

        assert_eq!(
            app.world().resource::<SpinBoxDirections>().0,
            vec![(spinbox, SpinBoxDirection::Decrement)]
        );
    }

    #[test]
    fn spinbox_emits_button_press_for_focused_descendant_arrow_keys() {
        let mut app = App::new();
        app.init_resource::<SpinBoxDirections>()
            .add_plugins((
                InputPlugin,
                InputFocusPlugin,
                InputDispatchPlugin,
                SpinBoxPlugin,
            ))
            .add_observer(
                |press: On<SpinBoxButtonPress>, mut directions: ResMut<SpinBoxDirections>| {
                    directions.0.push((press.entity, press.direction));
                },
            );
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();

        let spinbox = app.world_mut().spawn(SpinBox).id();
        let focused_child = app.world_mut().spawn(ChildOf(spinbox)).id();
        app.world_mut()
            .insert_resource(InputFocus::from_entity(focused_child));

        app.world_mut()
            .write_message(keyboard_input(KeyCode::ArrowRight))
            .unwrap();
        app.update();

        assert_eq!(
            app.world().resource::<SpinBoxDirections>().0,
            vec![(spinbox, SpinBoxDirection::Increment)]
        );
    }
}
