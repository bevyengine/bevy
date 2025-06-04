/// This module contains components that are used to track the interaction state of UI widgets.
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{
    component::Component,
    observer::Trigger,
    world::{DeferredWorld, OnInsert, OnRemove},
};

/// A component indicating that a widget is disabled and should be "grayed out".
/// This is used to prevent user interaction with the widget. It should not, however, prevent
/// the widget from being updated or rendered, or from acquiring keyboard focus.
///
/// For apps which support a11y: if a widget (such as a slider) contains multiple entities,
/// the `InteractionDisabled` component should be added to the root entity of the widget - the
/// same entity that contains the `AccessibilityNode` component. This will ensure that
/// the a11y tree is updated correctly.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(immutable)]
pub struct InteractionDisabled(pub bool);

impl InteractionDisabled {
    /// Returns whether the widget is currently disabled.
    pub fn get(&self) -> bool {
        self.0
    }
}

pub(crate) fn on_insert_disabled(
    trigger: Trigger<OnInsert, InteractionDisabled>,
    mut world: DeferredWorld,
) {
    let is_disabled = world
        .entity(trigger.target())
        .get::<InteractionDisabled>()
        .map(InteractionDisabled::get)
        .unwrap_or_default();
    let mut entity = world.entity_mut(trigger.target());
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        if is_disabled {
            accessibility.set_disabled();
        } else {
            accessibility.clear_disabled();
        }
    }
}

pub(crate) fn on_remove_disabled(
    trigger: Trigger<OnRemove, InteractionDisabled>,
    mut world: DeferredWorld,
) {
    let mut entity = world.entity_mut(trigger.target());
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.clear_disabled();
    }
}

/// Component that indicates whether a button or widget is currently in a pressed or "held down"
/// state.
#[derive(Component, Default, Debug)]
#[component(immutable)]
pub struct Depressed(pub bool);

impl Depressed {
    /// Returns whether the button is currently depressed.
    pub fn get(&self) -> bool {
        self.0
    }
}

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Default, Debug)]
pub struct Checked(pub bool);

impl Checked {
    /// Returns whether the checkbox or radio button is currently checked.
    pub fn get(&self) -> bool {
        self.0
    }
}

pub(crate) fn on_insert_checked(trigger: Trigger<OnInsert, Checked>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    let checked = entity.get::<Checked>().unwrap().get();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(match checked {
            true => accesskit::Toggled::True,
            false => accesskit::Toggled::False,
        });
    }
}

pub(crate) fn on_remove_checked(trigger: Trigger<OnRemove, Checked>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(trigger.target());
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(accesskit::Toggled::False);
    }
}
