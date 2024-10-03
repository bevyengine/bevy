use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, HierarchyQueryExt};
use bevy_picking::{
    events::{Down, Pointer, Up},
    focus::HoverMap,
    pointer::PointerId,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The data required to create a buttonlike widget that can respond to hovering and being clicked / pressed.
///
/// Information about whether or not this button is currently pressed or hovered is gathered from
/// user input, processed via [`bevy_picking`] and read in the [`determine_button_interaction`] system.
///
/// See [`FocusPolicy`](crate::focus::FocusPolicy) to configure whether or not the button should block interactions with lower nodes.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq)]
pub struct Button {
    /// Whether the button is currently pressed.
    ///
    /// This is `true` as long as the button is pressed, but only while the pointer
    /// is over the button.
    ///
    /// This value is set by the [`button_down_observer`] and [`button_up_observer`] [`Observer`]s.
    pub pressed: bool,
    /// Whether the button is currently hovered.
    ///
    /// This is only `true` while the pointer is over this button,
    /// and will not be set if there is no pointer.
    ///
    /// This value is set by the [`update_hover_status`] system.
    pub hovered: bool,
}

/// Updates the [`Button::hovered`] field based on the current [`HoverMap`] data.
///
/// Only [`PointerId::Mouse`] data is considered.
/// A button will be considered hovered if the mouse is over the button itself or any of its descendants.
pub fn update_hover_status(
    mut button_query: Query<(Entity, &mut Button)>,
    hover_map: Res<HoverMap>,
    children_query: Query<&Children>,
) {
    let Some(map) = hover_map.get(&PointerId::Mouse) else {
        // If no appropriate hover map data is found, we can assume that no buttons are hovered
        for (_, mut button) in button_query.iter_mut() {
            // Avoid triggering change detection spuriously
            // We can't use `set_if_neq` here because we're only looking at a single component
            if button.hovered {
                button.hovered = false;
            }
        }

        return;
    };

    // PERF: it might be faster to iterate over the smaller hover map and then propagate hovering upwards.
    // If this system starts showing up in profiles, try that.
    for (entity, mut button) in button_query.iter_mut() {
        let is_hovered = if map.contains_key(&entity) {
            true
        } else {
            let mut hovering_over_descendant = false;

            for descendant in children_query.iter_descendants(entity) {
                if map.contains_key(&descendant) {
                    hovering_over_descendant = true;
                    break;
                }
            }
            hovering_over_descendant
        };

        // Avoid triggering change detection spuriously
        // We can't use `set_if_neq` here because we're only looking at a single component
        if button.hovered != is_hovered {
            button.hovered = is_hovered;
        }
    }
}

/// An observer that watches for [`Pointer<Down>`] events and sets the [`Button::pressed`] field to `true`.
pub fn button_down_observer(trigger: Trigger<Pointer<Down>>, mut button_query: Query<&mut Button>) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.pressed = true;
    }
}

/// An observer that watches for [`Pointer<Up>`] events and sets the [`Button::pressed`] field to `false`.
pub fn button_up_observer(trigger: Trigger<Pointer<Up>>, mut button_query: Query<&mut Button>) {
    if let Ok(mut button) = button_query.get_mut(trigger.entity()) {
        button.pressed = false;
    }
}
