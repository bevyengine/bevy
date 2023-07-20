use crate::{Interaction, UiStack};
use bevy_a11y::Focus;
use bevy_ecs::prelude::Component;
use bevy_ecs::query::With;
use bevy_ecs::system::{Local, Query, Res, ResMut, Resource};
use bevy_ecs::{change_detection::DetectChangesMut, entity::Entity};
use bevy_input::{prelude::KeyCode, Input};
use bevy_reflect::Reflect;
use bevy_render::view::ComputedVisibility;

/// A component that represents if a UI element is focused.
#[derive(Reflect, Component, Clone, Debug, Default, Eq, PartialEq)]
pub struct Focusable {
    focus_state: FocusState,
}

impl Focusable {
    /// The entity is currently focused, similar to the `:focus` css pseudo-class.
    /// To check if the focus has been achieved through keyboard navigation, see [`Focusable::is_focus_visible`].
    pub fn is_focused(&self) -> bool {
        matches!(self.focus_state, FocusState::Focused { .. })
    }

    /// Focus has been reached through keyboard navigation and so a focus style should be displayed.
    /// This is similar to the `:focus-visible` pseudo-class in css.
    pub fn is_focus_visible(&self) -> bool {
        matches!(self.focus_state, FocusState::Focused { visible: true })
    }
}

#[derive(Reflect, Clone, Debug, Default, Eq, PartialEq)]
enum FocusState {
    /// Entity is not focused
    #[default]
    None,
    /// Entity is focused
    Focused {
        /// Focus has been reached through keyboard navigation and so a focus style should be displayed.
        /// This is similar to the `:focus-visible` pseudo-class in css.
        visible: bool,
    },
}

/// Resource for the configuration of keyboard navigation of UI with <kbd>tab</kbd>.
#[derive(Resource)]
pub struct KeyboardNavigationInput {
    pub enabled: bool,
}

impl Default for KeyboardNavigationInput {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Should the [`keyboard_navigation_system`] run?
pub(crate) fn tab_pressed(
    keyboard_input: Res<Input<KeyCode>>,
    keyboard_navigation: Res<KeyboardNavigationInput>,
) -> bool {
    keyboard_navigation.enabled && keyboard_input.just_pressed(KeyCode::Tab)
}

/// The system updates the [`Focus`] resource when the user uses keyboard navigation with <kbd>tab</kbd> or <kbd>shift</kbd> + <kbd>tab</kbd>.
///
/// Entities can be focused if [`ComputedVisibility`] is visible and they have the [`Focusable`] component.
pub(crate) fn keyboard_navigation_system(
    mut focus: ResMut<Focus>,
    mut interactions: Query<&mut Interaction>,
    focusables: Query<&ComputedVisibility, With<Focusable>>,
    keyboard_input: Res<Input<KeyCode>>,
    ui_stack: Res<UiStack>,
) {
    let reverse_order =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);

    let can_focus = |entity: &&Entity| {
        focusables
            .get(**entity)
            .map_or(false, |computed_visibility| {
                computed_visibility.is_visible()
            })
    };

    let ui_nodes = &ui_stack.uinodes;

    // Current index of the focused entity within the ui nodes list.
    let current_index = ui_nodes
        .iter()
        .position(|&ui_node| Some(ui_node) == focus.entity);

    let new_focus = if reverse_order {
        // Start with the entity before the current focused or at the end of the list
        let first_index = current_index.unwrap_or_default();

        let before = ui_nodes.iter().take(first_index);
        let after = ui_nodes.iter().skip(first_index);
        let mut wrapped = before.rev().chain(after.rev());
        wrapped.find(can_focus).copied()
    } else {
        // Start with the entity after the current focused or at the start of the list
        let first_index = current_index.map(|index| index + 1).unwrap_or_default();

        let after = ui_nodes.iter().skip(first_index);
        let before = ui_nodes.iter().take(first_index);
        let mut wrapped = after.chain(before);
        wrapped.find(can_focus).copied()
    };

    // Reset the clicked state
    if new_focus != focus.entity {
        if let Some(mut interaction) = focus
            .entity
            .and_then(|entity| interactions.get_mut(entity).ok())
        {
            if *interaction == Interaction::Pressed {
                *interaction = Interaction::None;
            }
        }
    }

    focus.set_if_neq(Focus {
        entity: new_focus,
        focus_visible: true,
    });
}

/// Change the [`FocusState`] for the specified entity
fn set_focus_state(
    entity: Option<Entity>,
    focusable: &mut Query<&mut Focusable>,
    focus_state: FocusState,
) {
    if let Some(mut focusable) = entity.and_then(|entity| focusable.get_mut(entity).ok()) {
        focusable.set_if_neq(Focusable { focus_state });
    }
}

/// Modify the [`FocusState`] of the [`Focusable`] component, based on the [`Focus`] resource.
pub(crate) fn update_focused_state(
    mut focusable: Query<&mut Focusable>,
    focus: Res<Focus>,
    mut old_focused_entity: Local<Option<Entity>>,
) {
    let new_focused_entity = focus.entity;

    // Remove the interaction from the last focused entity
    if *old_focused_entity != new_focused_entity {
        set_focus_state(*old_focused_entity, &mut focusable, FocusState::None);
    }

    let new_state = FocusState::Focused {
        visible: focus.focus_visible,
    };
    // Set the focused interaction on the newly focused entity
    set_focus_state(new_focused_entity, &mut focusable, new_state);

    *old_focused_entity = new_focused_entity;
}

/// Should the [`keyboard_click`] system run?
pub(crate) fn trigger_click(keyboard_input: Res<Input<KeyCode>>) -> bool {
    keyboard_input.just_pressed(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::Return)
}

/// Trigger the [`Focus`] entity to be clicked.
pub(crate) fn keyboard_click(mut interactions: Query<&mut Interaction>, focus: Res<Focus>) {
    if let Some(mut interaction) = focus
        .entity
        .and_then(|entity| interactions.get_mut(entity).ok())
    {
        interaction.set_if_neq(Interaction::Pressed);
    }
}

/// Should the [`end_keyboard_click`] system run?
pub(crate) fn trigger_click_end(keyboard_input: Res<Input<KeyCode>>) -> bool {
    keyboard_input.just_released(KeyCode::Space) || keyboard_input.just_released(KeyCode::Return)
}

/// Reset the clicked state.
pub(crate) fn end_keyboard_click(mut interactions: Query<&mut Interaction>) {
    interactions.for_each_mut(|mut interaction| {
        if *interaction == Interaction::Pressed {
            // The click was triggered by the keyboard, so it doesn't make sense to go to `Interaction::Hovered`.
            *interaction = Interaction::None;
        }
    });
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::{ButtonBundle, NodeBundle};
    use crate::{ui_stack_system, Focusable};
    use bevy_ecs::prelude::*;
    use bevy_ecs::system::CommandQueue;
    use bevy_hierarchy::BuildChildren;
    use bevy_utils::default;

    #[test]
    fn keyboard_navigation() {
        let mut world = World::default();

        let mut schedule = Schedule::new();
        schedule.add_systems((
            ui_stack_system,
            keyboard_navigation_system.after(ui_stack_system),
            update_focused_state.after(keyboard_navigation_system),
        ));

        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);

        commands.init_resource::<bevy_a11y::Focus>();
        commands.init_resource::<Input<KeyCode>>();
        commands.init_resource::<UiStack>();

        let mut children = Vec::new();
        commands
            .spawn(NodeBundle::default())
            .with_children(|parent| {
                for _child in 0..3 {
                    children.push(
                        parent
                            .spawn(ButtonBundle {
                                computed_visibility: ComputedVisibility::VISIBLE,
                                ..default()
                            })
                            .id(),
                    );
                }
            });
        queue.apply(&mut world);

        schedule.run(&mut world);
        assert!(
            world.get::<Focusable>(children[0]).unwrap().is_focused(),
            "navigation should start at the first button"
        );
        schedule.run(&mut world);
        assert!(
            world.get::<Focusable>(children[1]).unwrap().is_focused(),
            "navigation should go to the second button"
        );

        // Simulate pressing shift
        let mut keyboard_input = world
            .get_resource_mut::<Input<KeyCode>>()
            .expect("keyboard input resource");
        keyboard_input.press(KeyCode::ShiftLeft);

        schedule.run(&mut world);
        assert!(
            world.get::<Focusable>(children[0]).unwrap().is_focused(),
            "backwards navigation"
        );
        schedule.run(&mut world);
        assert!(
            world.get::<Focusable>(children[2]).unwrap().is_focused(),
            "navigation should loop around"
        );
    }
}
