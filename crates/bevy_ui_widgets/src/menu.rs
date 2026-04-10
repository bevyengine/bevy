//! Standard widget components for popup menus.
//!
//! Generally menus are structured as follows: there's a "menu" entity which is a container for a
//! "menu button" and a "menu popup". The popup may be pre-rendered and hidden while closed, or
//! it can be dynamically spawned on open and despawned on close - it's up to the widget implementer
//! to decide how they want to manage the popup.
//!
//! The popup should have a [`MenuPopup`] component. The menu button should have a [`MenuButton`]
//! component. The top level menu entity does not have any special component, but should have an
//! observer for menu events. The menu entity receives these events which bubble upward from both
//! the button and the popup. These events control the state of the menu (open and close) as well
//! as help manage focus.
//!
//! There's a tight coupling between menus and input focus: in order to detect clicks outside
//! the popup box (which cause the menu to close), we look for focus changes. This means that menu
//! popups only remain open as long as some child of the popup has focus. This also means that
//! when the popup first opens, focus must be set to a child (usually the first or last menu item
//! depending on what action caused the menu to open). Because opening the popup may not be instant,
//! especially if it's a queued BSN spawn, we can't set focus directly via the focus API. Instead,
//! you can insert a [`MenuFocusState`] component on the popup which will automatically focus
//! the appropriate child item when spawning is complete.
//!
//! Pressing the ESC key also closes the menu, but in that case focus reverts back to the menu
//! button.
//!
//! Finally, there's no rule against the menu entity having additional children besides the button
//! and the popup; for example, for something like a combo box widget, you might have a text input
//! widget that is a sibling of the menu button, both of which are contained inside a decorative
//! frame.

use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    observer::On,
    query::{Has, With},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
};
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    ButtonState,
};
use bevy_input_focus::{
    tab_navigation::{NavAction, TabGroup, TabNavigation},
    FocusedInput, InputFocus,
};
use bevy_log::warn;
use bevy_picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy_ui::{widget::Button, InteractionDisabled, Pressed};

use crate::{Activate, ActivateOnPress};

/// Action type for [`MenuEvent`].
#[derive(Clone, Copy, Debug)]
pub enum MenuAction {
    /// Indicates we want to open the menu, if it is not already open, and focus the first or
    /// last item depending on the [`NavAction`].
    Open(NavAction),
    /// Open the menu if it's closed, close it if it's open. Generally sent from a menu button.
    Toggle,
    /// Close the entire menu stack.
    CloseAll,
    /// Set focus to the menu button or other owner of the popup stack. This happens when
    /// the menu is closed by pressing the escape key.
    FocusRoot,
}

/// Event used to control the state of the open menu. This bubbles upwards from the menu items
/// and the menu container, through the portal relation, and to the menu owner entity.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuEvent {
    /// The [`MenuItem`] or [`MenuPopup`] that triggered this event.
    #[event_target]
    pub source: Entity,

    /// The desired action in response to this event.
    pub action: MenuAction,
}

/// Specifies the layout direction of the menu, for keyboard navigation
#[derive(Default, Debug, Clone, PartialEq)]
pub enum MenuLayout {
    /// A vertical stack. Up and down arrows to move between items.
    #[default]
    Column,
    /// A horizontal row. Left and right arrows to move between items.
    Row,
    /// A 2D grid. Arrow keys are not mapped, you'll need to write your own observer.
    Grid,
}

/// Component that defines a popup menu container.
///
/// Menus are automatically dismissed when the user clicks outside the menu bounds. Unlike a modal
/// dialog, where the click event is intercepted, we don't want to actually prevent the click event
/// from triggering its normal action. The easiest way to detect this kind of click is to look for
/// keyboard focus loss. When a menu is opened, one of its children will gain focus, and the menu
/// remains open so long as at least one descendant is focused. Arrow keys can be used to navigate
/// between menu items.
///
/// This means that popup menu *must* contain at least one focusable entity. It also means that two
/// menus cannot be displayed at the same time unless one is an ancestor of the other.
///
/// Some care needs to be taken in implementing a menu button: we normally want menu buttons to
/// toggle the open state of the menu; but clicking on the button will cause focus loss which means
/// that the menu will always be closed by the time the click event is processed.
#[derive(Component, Debug, Default, Clone)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::MenuListPopup)),
    TabGroup::modal()
)]
#[require(MenuFocusState::Closed)]
pub struct MenuPopup {
    /// The layout orientation of the menu
    pub layout: MenuLayout,
}

/// Component that defines a menu item.
#[derive(Component, Debug, Clone, Default)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))]
pub struct MenuItem;

/// Component used to manage focus on the popup. Menu popups remain open only so long as they
/// contain focus.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub enum MenuFocusState {
    /// A newly opened menu, which needs to have focus set to the first or last item depending on
    /// [`NavAction`].
    Opening(NavAction),
    /// Menu is open, and focus is set to an item within the menu
    Open,
    /// Menu is no longer visible, and can be cleaned up.
    #[default]
    Closed,
}

fn menu_acquire_focus(
    mut q_menus: Query<(Entity, &mut MenuFocusState), With<MenuPopup>>,
    mut focus: ResMut<InputFocus>,
    tab_navigation: TabNavigation,
) {
    for (menu, mut menu_focus) in q_menus.iter_mut() {
        // When a menu is spawned, attempt to find the first focusable menu item, and set focus
        // to it.
        if let MenuFocusState::Opening(nav) = *menu_focus {
            match tab_navigation.initialize(menu, nav) {
                Ok(next) => {
                    *menu_focus = MenuFocusState::Open;
                    focus.set(next);
                }
                Err(e) => {
                    warn!(
                        "No focusable menu items for popup menu: {}, error: {:?}",
                        menu, e
                    );
                }
            }
        }
    }
}

fn menu_on_lose_focus(
    mut q_menus: Query<(Entity, &mut MenuFocusState), With<MenuPopup>>,
    q_parent: Query<&ChildOf>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    // Close any menu which doesn't contain the focus entity.
    for (menu, mut menu_focus) in q_menus.iter_mut() {
        match *menu_focus {
            MenuFocusState::Opening(_) | MenuFocusState::Open => {
                // TODO: Change this logic when we support submenus. Don't want to send multiple close
                // events. Perhaps what we can do is add `MenuLostFocus` to the whole stack.
                let contains_focus = match focus.get() {
                    Some(focus_ent) => {
                        focus_ent == menu
                            || q_parent.iter_ancestors(focus_ent).any(|ent| ent == menu)
                    }
                    None => false,
                };

                if !contains_focus {
                    *menu_focus = MenuFocusState::Closed;
                    commands.trigger(MenuEvent {
                        source: menu,
                        action: MenuAction::CloseAll,
                    });
                }
            }

            _ => {}
        }
    }
}

fn menu_on_key_event(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_item: Query<Has<InteractionDisabled>, With<MenuItem>>,
    q_popup: Query<&MenuPopup>,
    tab_navigation: TabNavigation,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if let Ok(disabled) = q_item.get(ev.focused_entity) {
        if !disabled {
            let event = &ev.event().input;
            let entity = ev.event().focused_entity;
            if !event.repeat && event.state == ButtonState::Pressed {
                match event.key_code {
                    // Activate the item and close the popup
                    KeyCode::Enter | KeyCode::Space => {
                        ev.propagate(false);
                        // Trigger the action for this menu item.
                        commands.trigger(Activate { entity });
                        // Set the focus to the menu button.
                        commands.trigger(MenuEvent {
                            source: entity,
                            action: MenuAction::FocusRoot,
                        });
                        // Close the stack
                        commands.trigger(MenuEvent {
                            source: entity,
                            action: MenuAction::CloseAll,
                        });
                    }

                    _ => (),
                }
            }
        }
    } else if let Ok(menu) = q_popup.get(ev.focused_entity) {
        let event = &ev.event().input;
        if !event.repeat && event.state == ButtonState::Pressed {
            match event.key_code {
                // Close the popup
                KeyCode::Escape => {
                    ev.propagate(false);
                    // Close the stack
                    commands.trigger(MenuEvent {
                        source: ev.focused_entity,
                        action: MenuAction::CloseAll,
                    });
                    // Set the focus to the menu button.
                    commands.trigger(MenuEvent {
                        source: ev.focused_entity,
                        action: MenuAction::FocusRoot,
                    });
                }

                // Focus the adjacent item in the up direction
                KeyCode::ArrowUp if menu.layout == MenuLayout::Column => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::Previous) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                // Focus the adjacent item in the down direction
                KeyCode::ArrowDown if menu.layout == MenuLayout::Column => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::Next) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                // Focus the adjacent item in the left direction
                KeyCode::ArrowLeft if menu.layout == MenuLayout::Row => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::Previous) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                // Focus the adjacent item in the right direction
                KeyCode::ArrowRight if menu.layout == MenuLayout::Row => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::Next) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                // Focus the first item
                KeyCode::Home => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::First) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                // Focus the last item
                KeyCode::End => {
                    ev.propagate(false);
                    if let Ok(next) = tab_navigation.navigate(&focus, NavAction::Last) {
                        focus.set(next);
                    } else {
                        focus.clear();
                    }
                }

                _ => (),
            }
        }
    }
}

fn menu_item_on_pointer_click(
    mut ev: On<Pointer<Click>>,
    mut q_state: Query<(Has<Pressed>, Has<InteractionDisabled>), With<MenuItem>>,
    mut commands: Commands,
) {
    if let Ok((pressed, disabled)) = q_state.get_mut(ev.entity) {
        ev.propagate(false);
        if pressed && !disabled {
            // Trigger the menu action.
            commands.trigger(Activate { entity: ev.entity });
            // Set the focus to the menu button.
            commands.trigger(MenuEvent {
                source: ev.entity,
                action: MenuAction::FocusRoot,
            });
            // Close the stack
            commands.trigger(MenuEvent {
                source: ev.entity,
                action: MenuAction::CloseAll,
            });
        }
    }
}

fn menu_item_on_pointer_down(
    mut ev: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<MenuItem>>,
    mut commands: Commands,
) {
    if let Ok((item, disabled, pressed)) = q_state.get_mut(ev.entity) {
        ev.propagate(false);
        if !disabled && !pressed {
            commands.entity(item).insert(Pressed);
        }
    }
}

fn menu_item_on_pointer_up(
    mut ev: On<Pointer<Release>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<MenuItem>>,
    mut commands: Commands,
) {
    if let Ok((item, disabled, pressed)) = q_state.get_mut(ev.entity) {
        ev.propagate(false);
        if !disabled && pressed {
            commands.entity(item).remove::<Pressed>();
        }
    }
}

fn menu_item_on_pointer_drag_end(
    mut ev: On<Pointer<DragEnd>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<MenuItem>>,
    mut commands: Commands,
) {
    if let Ok((item, disabled, pressed)) = q_state.get_mut(ev.entity) {
        ev.propagate(false);
        if !disabled && pressed {
            commands.entity(item).remove::<Pressed>();
        }
    }
}

fn menu_item_on_pointer_cancel(
    mut ev: On<Pointer<Cancel>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<MenuItem>>,
    mut commands: Commands,
) {
    if let Ok((item, disabled, pressed)) = q_state.get_mut(ev.entity) {
        ev.propagate(false);
        if !disabled && pressed {
            commands.entity(item).remove::<Pressed>();
        }
    }
}

/// Headless menu button widget. This is meant to be combined with the `Button` component, and
/// adds a few more key codes - arrow keys to open the popup.
#[derive(Component, Default, Debug, Clone)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::Button)),
    Button,
    ActivateOnPress
)]
pub struct MenuButton;

fn menubutton_on_activate(
    activate: On<Activate>,
    q_menu_button: Query<Has<InteractionDisabled>, With<MenuButton>>,
    mut commands: Commands,
) {
    if let Ok(disabled) = q_menu_button.get(activate.entity)
        && !disabled
    {
        commands.trigger(MenuEvent {
            source: activate.entity,
            action: MenuAction::Toggle,
        });
    }
}

fn menubutton_on_key_event(
    mut event: On<FocusedInput<KeyboardInput>>,
    q_menu_button: Query<Has<InteractionDisabled>, With<MenuButton>>,
    mut commands: Commands,
) {
    if let Ok(disabled) = q_menu_button.get(event.focused_entity) {
        event.propagate(false);
        if disabled {
            return;
        }
        let input_event = &event.input;
        if !input_event.repeat && input_event.state == ButtonState::Pressed {
            match input_event.key_code {
                // Focus the last item in the menu
                KeyCode::ArrowUp | KeyCode::ArrowLeft => {
                    event.propagate(false);
                    commands.trigger(MenuEvent {
                        action: MenuAction::Open(NavAction::Last),
                        source: event.focused_entity,
                    });
                }

                // Focus the first item in the menu
                KeyCode::ArrowDown | KeyCode::ArrowRight => {
                    event.propagate(false);
                    commands.trigger(MenuEvent {
                        action: MenuAction::Open(NavAction::First),
                        source: event.focused_entity,
                    });
                }

                _ => {}
            }
        }
    }
}

/// Plugin that adds the observers for the [`MenuItem`] component.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (menu_acquire_focus, menu_on_lose_focus).chain())
            .add_observer(menu_on_key_event)
            .add_observer(menu_item_on_pointer_down)
            .add_observer(menu_item_on_pointer_up)
            .add_observer(menu_item_on_pointer_click)
            .add_observer(menu_item_on_pointer_drag_end)
            .add_observer(menu_item_on_pointer_cancel)
            .add_observer(menubutton_on_key_event)
            .add_observer(menubutton_on_activate);
    }
}
