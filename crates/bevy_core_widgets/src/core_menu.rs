//! Core widget components for menus and menu buttons.

use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::{EntityEvent, Event},
    hierarchy::ChildOf,
    lifecycle::Add,
    observer::On,
    query::{Has, With},
    system::{Commands, Query, ResMut},
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{
    tab_navigation::{NavAction, TabGroup, TabNavigation},
    AcquireFocus, FocusedInput, InputFocus,
};
use bevy_log::warn;
use bevy_ui::InteractionDisabled;
use bevy_window::PrimaryWindow;

use crate::{Callback, Notify};

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
    /// Close the entire menu stack. The boolean argument indicates whether we want to retain
    /// focus on the menu owner (the menu button). Whether this is true will depend on the reason
    /// for closing: a click on the background should not restore focus to the button.
    CloseAll(bool),
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

/// Component that defines a popup menu container.
///
/// A popup menu *must* contain at least one focusable entity. The first such entity will acquire
/// focus when the popup is spawned; arrow keys can be used to navigate between menu items. If no
/// descendant of the menu has focus, the menu will automatically close. This rule has several
/// consequences:
///
/// * Clicking on another widget or empty space outside the menu will cause the menu to close.
/// * Two menus cannot be displayed at the same time unless one is an ancestor of the other.
#[derive(Component, Debug)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::MenuListPopup)),
    TabGroup::modal()
)]
pub struct CoreMenuPopup;

/// Component that defines a menu item.
#[derive(Component, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))]
pub struct CoreMenuItem {
    /// Callback to invoke when the menu item is clicked, or when the `Enter` or `Space` key
    /// is pressed while the item is focused.
    pub on_activate: Callback,
}

fn menu_on_spawn(
    ev: On<Add, CoreMenuPopup>,
    mut focus: ResMut<InputFocus>,
    tab_navigation: TabNavigation,
) {
    // When a menu is spawned, attempt to find the first focusable menu item, and set focus
    // to it.
    if let Ok(next) = tab_navigation.initialize(ev.target(), NavAction::First) {
        focus.0 = Some(next);
    } else {
        warn!("No focusable menu items for popup menu: {}", ev.target());
    }
}

fn menu_on_key_event(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_item: Query<(&CoreMenuItem, Has<InteractionDisabled>)>,
    q_menu: Query<&CoreMenuPopup>,
    mut commands: Commands,
) {
    if let Ok((menu_item, disabled)) = q_item.get(ev.target()) {
        if !disabled {
            let event = &ev.event().input;
            if !event.repeat && event.state == ButtonState::Pressed {
                match event.key_code {
                    // Activate the item and close the popup
                    KeyCode::Enter | KeyCode::Space => {
                        ev.propagate(false);
                        commands.notify(&menu_item.on_activate);
                        commands.trigger_targets(MenuEvent::CloseAll(true), ev.target());
                    }

                    _ => (),
                }
            }
        }
    } else if let Ok(menu) = q_menu.get(ev.target()) {
        let event = &ev.event().input;
        if !event.repeat && event.state == ButtonState::Pressed {
            match event.key_code {
                // Close the popup
                KeyCode::Escape => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::CloseAll(true), ev.target());
                }

                // Focus the adjacent item in the up direction
                KeyCode::ArrowUp => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusUp, ev.target());
                }

                // Focus the adjacent item in the down direction
                KeyCode::ArrowDown => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusDown, ev.target());
                }

                // Focus the adjacent item in the left direction
                KeyCode::ArrowLeft => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusLeft, ev.target());
                }

                // Focus the adjacent item in the right direction
                KeyCode::ArrowRight => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusRight, ev.target());
                }

                // Focus the first item
                KeyCode::Home => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusFirst, ev.target());
                }

                // Focus the last item
                KeyCode::End => {
                    ev.propagate(false);
                    commands.trigger_targets(MenuEvent::FocusLast, ev.target());
                }

                _ => (),
            }
        }
    }
}

fn menu_on_menu_event(
    mut ev: On<MenuEvent>,
    q_popup: Query<(), With<CoreMenuPopup>>,
    q_parent: Query<&ChildOf>,
    windows: Query<Entity, With<PrimaryWindow>>,
    mut commands: Commands,
) {
    if q_popup.contains(ev.target()) {
        match ev.event() {
            MenuEvent::Open => todo!(),
            MenuEvent::Close => {
                ev.propagate(false);
                commands.entity(ev.target()).despawn();
            }
            MenuEvent::CloseAll(retain_focus) => {
                // For CloseAll, find the root menu popup and despawn it
                // This will propagate the despawn to all child popups
                let root_menu = q_parent
                    .iter_ancestors(ev.target())
                    .filter(|&e| q_popup.contains(e))
                    .last()
                    .unwrap_or(ev.target());

                // Get the parent of the root menu and trigger an AcquireFocus event.
                if let Ok(root_parent) = q_parent.get(root_menu) {
                    if *retain_focus {
                        if let Ok(window) = windows.single() {
                            commands.trigger_targets(AcquireFocus { window }, root_parent.parent());
                        }
                    }
                }

                ev.propagate(false);
                commands.entity(root_menu).despawn();
            }
            MenuEvent::FocusFirst => todo!(),
            MenuEvent::FocusLast => todo!(),
            MenuEvent::FocusPrev => todo!(),
            MenuEvent::FocusNext => todo!(),
            MenuEvent::FocusUp => todo!(),
            MenuEvent::FocusDown => todo!(),
            MenuEvent::FocusLeft => todo!(),
            MenuEvent::FocusRight => todo!(),
        }
    }
}

/// Plugin that adds the observers for the [`CoreButton`] widget.
pub struct CoreMenuPlugin;

impl Plugin for CoreMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(menu_on_spawn)
            .add_observer(menu_on_key_event)
            .add_observer(menu_on_menu_event);
    }
}
