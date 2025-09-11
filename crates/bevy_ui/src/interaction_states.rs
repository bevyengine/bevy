/// This module contains components that are used to track the interaction state of UI widgets.
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{
    component::Component,
    lifecycle::{Add, Remove},
    observer::On,
    world::DeferredWorld,
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
pub struct InteractionDisabled;

pub(crate) fn on_add_disabled(add: On<Add, InteractionDisabled>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_disabled();
    }
}

pub(crate) fn on_remove_disabled(
    remove: On<Remove, InteractionDisabled>,
    mut world: DeferredWorld,
) {
    let mut entity = world.entity_mut(remove.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.clear_disabled();
    }
}

/// Component that indicates whether a button or widget is currently in a pressed or "held down"
/// state.
#[derive(Component, Default, Debug)]
pub struct Pressed;

/// Component that indicates that a widget can be checked.
#[derive(Component, Default, Debug)]
pub struct Checkable;

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Default, Debug)]
pub struct Checked;

pub(crate) fn on_add_checkable(add: On<Add, Checked>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    let checked = entity.get::<Checked>().is_some();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(match checked {
            true => accesskit::Toggled::True,
            false => accesskit::Toggled::False,
        });
    }
}

pub(crate) fn on_remove_checkable(add: On<Add, Checked>, mut world: DeferredWorld) {
    // Remove the 'toggled' attribute entirely.
    let mut entity = world.entity_mut(add.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.clear_toggled();
    }
}

pub(crate) fn on_add_checked(add: On<Add, Checked>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(accesskit::Toggled::True);
    }
}

pub(crate) fn on_remove_checked(remove: On<Remove, Checked>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(remove.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(accesskit::Toggled::False);
    }
}
