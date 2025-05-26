/// This module contains components that are used to track the interaction state of UI widgets.
///
// Note to implementers: This uses a combination of both marker components and newtype components
// containing a bool. Markers are used for one-way binding, "write-only" components
// (like `InteractionDisabled`) that relay instructions from the user to the framework, whereas
// newtype components are used to request state updates from the framework, which mutates the
// content of those components on update.
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{
    component::{Component, HookContext},
    world::DeferredWorld,
};

/// A marker component to indicate that a widget is disabled and should be "grayed out".
/// This is used to prevent user interaction with the widget. It should not, however, prevent
/// the widget from being updated or rendered, or from acquiring keyboard focus.
///
/// For apps which support a11y: if a widget (such as a slider) contains multiple entities,
/// the `InteractionDisabled` component should be added to the root entity of the widget - the
/// same entity that contains the `AccessibilityNode` component. This will ensure that
/// the a11y tree is updated correctly.
#[derive(Component, Debug, Clone, Copy)]
#[component(on_add = on_add_disabled, on_remove = on_remove_disabled)]
pub struct InteractionDisabled;

// Hook to set the a11y "disabled" state when the widget is disabled.
fn on_add_disabled(mut world: DeferredWorld, context: HookContext) {
    let mut entity = world.entity_mut(context.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_disabled();
    }
}

// Hook to remove the a11y "disabled" state when the widget is enabled.
fn on_remove_disabled(mut world: DeferredWorld, context: HookContext) {
    let mut entity = world.entity_mut(context.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.clear_disabled();
    }
}

/// Component that indicates whether a button or widget is currently in a pressed or "held down"
/// state.
#[derive(Component, Default, Debug)]
#[component(immutable)]
pub struct Depressed(pub bool);

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Default, Debug)]
#[component(immutable, on_add = on_add_checked, on_replace = on_add_checked)]
pub struct Checked(pub bool);

// Hook to set the a11y "checked" state when the checkbox is added.
fn on_add_checked(mut world: DeferredWorld, context: HookContext) {
    let mut entity = world.entity_mut(context.entity);
    let checked = entity.get::<Checked>().unwrap().0;
    let mut accessibility = entity.get_mut::<AccessibilityNode>().unwrap();
    accessibility.set_toggled(match checked {
        true => accesskit::Toggled::True,
        false => accesskit::Toggled::False,
    });
}
