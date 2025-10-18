//! Core widget components for menus and menu buttons.

use accesskit::Role;
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::EntityEvent,
    hierarchy::ChildOf,
    observer::On,
    query::{Has, With, Without},
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    template::GetTemplate,
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
use bevy_ui::{InteractionDisabled, Pressed};

use crate::Activate;

/// Action type for [`MenuEvent`].
#[derive(Clone, Copy, Debug)]
pub enum MenuAction {
    /// Indicates we want to open the menu, if it is not already open.
    Open,
    /// Open the menu if it's closed, close it if it's open. Generally sent from a menu button.
    Toggle,
    /// Close the menu and despawn it. Despawning may not happen immediately if there is a closing
    /// transition animation.
    Close,
    /// Close the entire menu stack.
    CloseAll,
    /// Set focus to the menu button or other owner of the popup stack. This happens when
    /// the escape key is pressed.
    FocusRoot,
}

/// Event used to control the state of the open menu. This bubbles upwards from the menu items
/// and the menu container, through the portal relation, and to the menu owner entity.
///
/// Focus navigation: the menu may be part of a composite of multiple menus such as a menu bar.
/// This means that depending on direction, focus movement may move to the next menu item, or
/// the next menu. This also means that different events will often be handled at different
/// levels of the hierarchy - some being handled by the popup, and some by the popup's owner.
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
/// A popup menu *must* contain at least one focusable entity. The first such entity will acquire
/// focus when the popup is spawned; arrow keys can be used to navigate between menu items. If no
/// descendant of the menu has focus, the menu will automatically close. This rule has several
/// consequences:
///
/// * Clicking on another widget or empty space outside the menu will cause the menu to close.
/// * Two menus cannot be displayed at the same time unless one is an ancestor of the other.
#[derive(Component, Debug, Default, Clone)]
#[require(
    AccessibilityNode(accesskit::Node::new(Role::MenuListPopup)),
    TabGroup::modal()
)]
#[require(MenuAcquireFocus)]
pub struct MenuPopup {
    /// The layout orientation of the menu
    pub layout: MenuLayout,
}

/// Component that defines a menu item.
#[derive(Component, Debug, Clone, GetTemplate)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))]
pub struct MenuItem;

/// Marker component that indicates that we need to set focus to the first menu item.
#[derive(Component, Debug, Default)]
struct MenuAcquireFocus;

/// Component that indicates that the menu is closing.
#[derive(Component, Debug, Default)]
struct MenuClosing;

fn menu_acquire_focus(
    q_menus: Query<Entity, (With<MenuPopup>, With<MenuAcquireFocus>)>,
    mut focus: ResMut<InputFocus>,
    tab_navigation: TabNavigation,
    mut commands: Commands,
) {
    for menu in q_menus.iter() {
        // When a menu is spawned, attempt to find the first focusable menu item, and set focus
        // to it.
        match tab_navigation.initialize(menu, NavAction::First) {
            Ok(next) => {
                commands.entity(menu).remove::<MenuAcquireFocus>();
                focus.0 = Some(next);
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

fn menu_on_lose_focus(
    q_menus: Query<
        Entity,
        (
            With<MenuPopup>,
            Without<MenuAcquireFocus>,
            Without<MenuClosing>,
        ),
    >,
    q_parent: Query<&ChildOf>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    // Close any menu which doesn't contain the focus entity.
    for menu in q_menus.iter() {
        // TODO: Change this logic when we support submenus. Don't want to send multiple close
        // events. Perhaps what we can do is add `CoreMenuClosing` to the whole stack.
        let contains_focus = match focus.0 {
            Some(focus_ent) => {
                focus_ent == menu || q_parent.iter_ancestors(focus_ent).any(|ent| ent == menu)
            }
            None => false,
        };

        if !contains_focus {
            commands.entity(menu).insert(MenuClosing);
            commands.trigger(MenuEvent {
                source: menu,
                action: MenuAction::CloseAll,
            });
        }
    }
}

fn menu_on_key_event(
    mut ev: On<FocusedInput<KeyboardInput>>,
    q_item: Query<Has<InteractionDisabled>, With<MenuItem>>,
    q_menu: Query<&MenuPopup>,
    tab_navigation: TabNavigation,
    mut focus: ResMut<InputFocus>,
    mut commands: Commands,
) {
    if let Ok(disabled) = q_item.get(ev.focused_entity) {
        if !disabled {
            let event = &ev.event().input;
            if !event.repeat && event.state == ButtonState::Pressed {
                match event.key_code {
                    // Activate the item and close the popup
                    KeyCode::Enter | KeyCode::Space => {
                        ev.propagate(false);
                        // Trigger the menu action
                        commands.trigger(Activate {
                            entity: ev.event().focused_entity,
                        });
                        // Set the focus to the menu button.
                        commands.trigger(MenuEvent {
                            source: ev.event().focused_entity,
                            action: MenuAction::FocusRoot,
                        });
                        // Close the stack
                        commands.trigger(MenuEvent {
                            source: ev.event().focused_entity,
                            action: MenuAction::CloseAll,
                        });
                    }

                    _ => (),
                }
            }
        }
    } else if let Ok(menu) = q_menu.get(ev.focused_entity) {
        let event = &ev.event().input;
        if !event.repeat && event.state == ButtonState::Pressed {
            match event.key_code {
                // Close the popup
                KeyCode::Escape => {
                    ev.propagate(false);
                    // Set the focus to the menu button.
                    commands.trigger(MenuEvent {
                        source: ev.focused_entity,
                        action: MenuAction::FocusRoot,
                    });
                    // Close the stack
                    commands.trigger(MenuEvent {
                        source: ev.focused_entity,
                        action: MenuAction::CloseAll,
                    });
                }

                // Focus the adjacent item in the up direction
                KeyCode::ArrowUp => {
                    if menu.layout == MenuLayout::Column {
                        ev.propagate(false);
                        focus.0 = tab_navigation.navigate(&focus, NavAction::Previous).ok();
                    }
                }

                // Focus the adjacent item in the down direction
                KeyCode::ArrowDown => {
                    if menu.layout == MenuLayout::Column {
                        ev.propagate(false);
                        focus.0 = tab_navigation.navigate(&focus, NavAction::Next).ok();
                    }
                }

                // Focus the adjacent item in the left direction
                KeyCode::ArrowLeft => {
                    if menu.layout == MenuLayout::Row {
                        ev.propagate(false);
                        focus.0 = tab_navigation.navigate(&focus, NavAction::Previous).ok();
                    }
                }

                // Focus the adjacent item in the right direction
                KeyCode::ArrowRight => {
                    if menu.layout == MenuLayout::Row {
                        ev.propagate(false);
                        focus.0 = tab_navigation.navigate(&focus, NavAction::Next).ok();
                    }
                }

                // Focus the first item
                KeyCode::Home => {
                    ev.propagate(false);
                    focus.0 = tab_navigation.navigate(&focus, NavAction::First).ok();
                }

                // Focus the last item
                KeyCode::End => {
                    ev.propagate(false);
                    focus.0 = tab_navigation.navigate(&focus, NavAction::Last).ok();
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

fn menu_on_menu_event(
    mut ev: On<MenuEvent>,
    q_popup: Query<(), With<MenuPopup>>,
    mut commands: Commands,
) {
    if q_popup.contains(ev.source) {
        if let MenuAction::Close = ev.event().action {
            ev.propagate(false);
            commands.entity(ev.source).despawn();
        }
    }
}

/// Plugin that adds the observers for the [`CoreMenuItem`] widget.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (menu_acquire_focus, menu_on_lose_focus).chain())
            .add_observer(menu_on_key_event)
            .add_observer(menu_on_menu_event)
            .add_observer(menu_item_on_pointer_down)
            .add_observer(menu_item_on_pointer_up)
            .add_observer(menu_item_on_pointer_click)
            .add_observer(menu_item_on_pointer_drag_end)
            .add_observer(menu_item_on_pointer_cancel);
    }
}
