//! Standard widget components for popup menus.

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
#[require(MenuAcquireFocus)]
pub struct MenuPopup {
    /// The layout orientation of the menu
    pub layout: MenuLayout,
}

/// Component that defines a menu item.
#[derive(Component, Debug, Clone)]
#[require(AccessibilityNode(accesskit::Node::new(Role::MenuItem)))]
pub struct MenuItem;

/// Marker component that indicates that we need to set focus to the first menu item.
#[derive(Component, Debug, Default)]
struct MenuAcquireFocus;

/// Component that indicates that the menu lost focus and is in the process of closing.
#[derive(Component, Debug, Default)]
struct MenuLostFocus;

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
            Without<MenuLostFocus>,
        ),
    >,
    q_parent: Query<&ChildOf>,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    // Close any menu which doesn't contain the focus entity.
    for menu in q_menus.iter() {
        // TODO: Change this logic when we support submenus. Don't want to send multiple close
        // events. Perhaps what we can do is add `MenuLostFocus` to the whole stack.
        let contains_focus = match focus.0 {
            Some(focus_ent) => {
                focus_ent == menu || q_parent.iter_ancestors(focus_ent).any(|ent| ent == menu)
            }
            None => false,
        };

        if !contains_focus {
            commands.entity(menu).insert(MenuLostFocus);
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
    if q_popup.contains(ev.source)
        && let MenuAction::Close = ev.event().action
    {
        ev.propagate(false);
        commands.entity(ev.source).despawn();
    }
}

/// Headless menu button widget. This is similar to a button, except for a few differences:
/// * It emits a menu toggle event when pressed or activated.
/// * It uses `Pointer<Press>` rather than click, so as to process the pointer event before
///   stealing focus from the menu.
#[derive(Component, Default, Debug)]
#[require(AccessibilityNode(accesskit::Node::new(Role::Button)))]
pub struct MenuButton;

fn menubutton_on_key_event(
    mut event: On<FocusedInput<KeyboardInput>>,
    q_state: Query<Has<InteractionDisabled>, With<MenuButton>>,
    mut commands: Commands,
) {
    if let Ok(disabled) = q_state.get(event.focused_entity)
        && !disabled
    {
        let input_event = &event.input;
        if !input_event.repeat
            && input_event.state == ButtonState::Pressed
            && (input_event.key_code == KeyCode::Enter || input_event.key_code == KeyCode::Space)
        {
            event.propagate(false);
            commands.trigger(MenuEvent {
                action: MenuAction::Toggle,
                source: event.focused_entity,
            });
        }
    }
}

fn menubutton_on_pointer_press(
    mut press: On<Pointer<Press>>,
    mut q_state: Query<(Entity, Has<InteractionDisabled>, Has<Pressed>), With<MenuButton>>,
    mut commands: Commands,
) {
    if let Ok((button, disabled, pressed)) = q_state.get_mut(press.entity) {
        press.propagate(false);
        if !disabled && !pressed {
            commands.trigger(MenuEvent {
                action: MenuAction::Toggle,
                source: button,
            });
        }
    }
}

/// Plugin that adds the observers for the [`MenuItem`] component.
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
            .add_observer(menu_item_on_pointer_cancel)
            .add_observer(menubutton_on_key_event)
            .add_observer(menubutton_on_pointer_press);
    }
}
