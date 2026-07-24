/// This module contains components that are used to track the interaction state of UI widgets.
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{
    change_detection::Mut,
    component::Component,
    lifecycle::{Add, Remove},
    observer::On,
    reflect::ReflectComponent,
    world::DeferredWorld,
};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// A component indicating that a widget is disabled and should be "grayed out".
/// This is used to prevent user interaction with the widget. It should not, however, prevent
/// the widget from being updated or rendered, or from acquiring keyboard focus.
///
/// For apps which support a11y: if a widget (such as a slider) contains multiple entities,
/// the `InteractionDisabled` component should be added to the root entity of the widget - the
/// same entity that contains the `AccessibilityNode` component. This will ensure that
/// the a11y tree is updated correctly.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
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
///
/// When this component is first inserted into a button or widget, its value is true.
/// When this button or widget is no longer being pressed, its value is false for a frame
/// before being removed from the entity. This enables change detection for when
/// this entity has transitioned from being pressed to not being pressed.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct Pressed(pub bool);

impl Pressed {
    /// Get whether the entity is currently being pressed.
    pub fn get(&self) -> bool {
        self.0
    }
}

impl Default for Pressed {
    fn default() -> Self {
        Self(true)
    }
}

/// Extension trait for `Option<&Pressed>` for a convenience method
/// to more easily checked the pressed state.
pub trait OptionPressedExt {
    fn is_pressed(&self) -> bool;
}

impl OptionPressedExt for Option<&Pressed> {
    fn is_pressed(&self) -> bool {
        self.is_some_and(|pressed| pressed.get())
    }
}

impl OptionPressedExt for Option<Mut<'_, Pressed>> {
    fn is_pressed(&self) -> bool {
        self.as_ref().is_some_and(|pressed| pressed.get())
    }
}

/// Component that indicates that a widget can be checked.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct Checkable;

/// Component that indicates whether a checkbox or radio button is in a checked state.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct Checked;

pub(crate) fn on_add_checkable(add: On<Add, Checkable>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    let checked = entity.get::<Checked>().is_some();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_toggled(match checked {
            true => accesskit::Toggled::True,
            false => accesskit::Toggled::False,
        });
    }
}

pub(crate) fn on_remove_checkable(add: On<Remove, Checkable>, mut world: DeferredWorld) {
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

/// Component that indicates that a widget can be selected. Similar to [`Checkable`], but works for
/// the ARIA "selected" state instead of "checked".
#[derive(Component, Default, Debug)]
pub struct Selectable;

/// Similar to [`Checked`], but works for the ARIA "selected" state instead of "checked".
#[derive(Component, Default, Debug, Clone)]
pub struct Selected;

pub(crate) fn on_add_selectable(add: On<Add, Selectable>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    let selected = entity.get::<Selected>().is_some();
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_selected(selected);
    }
}

pub(crate) fn on_remove_selectable(add: On<Add, Selectable>, mut world: DeferredWorld) {
    // Remove the 'toggled' attribute entirely.
    let mut entity = world.entity_mut(add.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.clear_selected();
    }
}

pub(crate) fn on_add_selected(add: On<Add, Selected>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(add.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_selected(true);
    }
}

pub(crate) fn on_remove_selected(remove: On<Remove, Selected>, mut world: DeferredWorld) {
    let mut entity = world.entity_mut(remove.entity);
    if let Some(mut accessibility) = entity.get_mut::<AccessibilityNode>() {
        accessibility.set_selected(false);
    }
}
