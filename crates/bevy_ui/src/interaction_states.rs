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
    entity::Entity,
    hierarchy::ChildOf,
    system::{Query, Res},
    world::DeferredWorld,
};
use bevy_picking::{hover::HoverMap, pointer::PointerId};

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
    let mut entt = world.entity_mut(context.entity);
    if let Some(mut accessibility) = entt.get_mut::<AccessibilityNode>() {
        accessibility.set_disabled();
    }
}

// Hook to remove the a11y "disabled" state when the widget is enabled.
fn on_remove_disabled(mut world: DeferredWorld, context: HookContext) {
    let mut entt = world.entity_mut(context.entity);
    if let Some(mut accessibility) = entt.get_mut::<AccessibilityNode>() {
        accessibility.clear_disabled();
    }
}

/// Component that indicates whether a button is currently pressed. This will be true while
/// a drag action is in progress.
#[derive(Component, Default, Debug)]
pub struct ButtonPressed(pub bool);

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Default, Debug)]
#[component(immutable, on_add = on_add_checked, on_replace = on_add_checked)]
pub struct Checked(pub bool);

// Hook to set the a11y "checked" state when the checkbox is added.
fn on_add_checked(mut world: DeferredWorld, context: HookContext) {
    let mut entt = world.entity_mut(context.entity);
    let checked = entt.get::<Checked>().unwrap().0;
    let mut accessibility = entt.get_mut::<AccessibilityNode>().unwrap();
    accessibility.set_toggled(match checked {
        true => accesskit::Toggled::True,
        false => accesskit::Toggled::False,
    });
}

/// Component which indicates that the entity is interested in knowing when the mouse is hovering
/// over it or any of its children. Using this component lets users use regular Bevy change
/// detection for hover enter and leave transitions instead of having to rely on observers or hooks.
///
/// TODO: This component and it's associated system isn't UI-specific, and could be moved to the
/// bevy_picking crate.
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct Hovering(pub bool);

// TODO: This should be registered as a system after the hover map is updated.
pub(crate) fn update_hover_states(
    hover_map: Option<Res<HoverMap>>,
    mut hovers: Query<(Entity, &mut Hovering)>,
    parent_query: Query<&ChildOf>,
) {
    let Some(hover_map) = hover_map else { return };
    let hover_set = hover_map.get(&PointerId::Mouse);
    for (entity, mut hoverable) in hovers.iter_mut() {
        let is_hovering = match hover_set {
            Some(map) => map.iter().any(|(ha, _)| {
                *ha == entity || parent_query.iter_ancestors(*ha).any(|e| e == entity)
            }),
            None => false,
        };
        if hoverable.0 != is_hovering {
            hoverable.0 = is_hovering;
        }
    }
}
