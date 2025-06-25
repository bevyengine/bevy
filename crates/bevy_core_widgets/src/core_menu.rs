//! Core widget components for menus and menu buttons.

use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{EntityEvent, Event},
    system::SystemId,
    traversal::Traversal,
};

use crate::portal::{PortalTraversal, PortalTraversalItem};

/// Event use to control the state of the open menu. This bubbles upwards from the menu items
/// and the menu container, through the portal relation, and to the menu owner entity.
///
/// Focus navigation: the menu may be part of a composite of multiple menus such as a menu bar.
/// This means that depending on direction, focus movement may move to the next menu item, or
/// the next menu. This also means that different events will often be handled at different
/// levels of the hierarchy - some being handled by the popup, and some by the popup's owner.
#[derive(Event, EntityEvent, Clone)]
pub enum MenuEvent {
    /// Indicates we want to open the menu, if it is not already open.
    Open,
    /// Close the menu and despawn it. Despawning may not happen immediately if there is a closing
    /// transition animation.
    Close,
    /// Move the input focs to the parent element. This usually happens as the menu is closing,
    /// although will not happen if the close was a result of clicking on the background.
    FocusParent,
    /// Move the input focus to the first child in the parent's hierarchy (Home).
    FocusFirst,
    /// Move the input focus to the last child in the parent's hierarchy (End).
    FocusLast,
    /// Move the input focus to the previous child in the parent's hierarchy (Shift-Tab).
    FocusPrev,
    /// Move the input focus to the next child in the parent's hierarchy (Tab).
    FocusNext,
    /// Move the input focus up (Arrow-Up).
    FocusUp,
    /// Move the input focus down (Arrow-Down).
    FocusDown,
    /// Move the input focus left (Arrow-Left).
    FocusLeft,
    /// Move the input focus right (Arrow-Right).
    FocusRight,
}

impl Traversal<MenuEvent> for PortalTraversal {
    fn traverse(item: Self::Item<'_, '_>, _event: &MenuEvent) -> Option<Entity> {
        let PortalTraversalItem {
            child_of,
            portal_child_of,
        } = item;

        // Send event to portal parent, if it has one.
        if let Some(portal_child_of) = portal_child_of {
            return Some(portal_child_of.parent());
        };

        // Send event to parent, if it has one.
        if let Some(child_of) = child_of {
            return Some(child_of.parent());
        };

        None
    }
}

/// Component that defines a popup menu container.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuListPopup)))]
pub struct CoreMenuPopup;

/// Component that defines a menu item.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))]
pub struct CoreMenuItem {
    /// Optional system to run when the menu item is clicked, or when the Enter or Space key
    /// is pressed while the item is focused.
    pub on_click: Option<SystemId>,
}
