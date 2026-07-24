//! This example illustrates how to create a context menu that changes the clear color

use bevy::{
    color::palettes::basic,
    ecs::{relationship::RelatedSpawner, spawn::SpawnWith},
    prelude::*,
    ui_widgets::{ListBox, ListItem, ValueChange},
};
use std::fmt::Debug;

/// event opening a new context menu at position `pos`
#[derive(Event)]
struct OpenContextMenu {
    pos: Vec2,
}

/// event will be sent to close currently open context menus
#[derive(Event)]
struct CloseContextMenus;

/// marker component identifying root of a context menu
#[derive(Component, Clone, Default)]
struct ContextMenu;

/// context menu item data storing what background color `Srgba` it activates
#[derive(Component, Clone, Default, PartialEq)]
struct ContextMenuItem(Srgba);

/// marker component for context item text
#[derive(Component, Clone, Default)]
struct ContextMenuItemText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(on_trigger_menu)
        .add_observer(on_trigger_close_menus)
        .add_observer(text_color_on_hover::<Out>(basic::WHITE.into()))
        .add_observer(text_color_on_hover::<Over>(basic::RED.into()))
        .run();
}

/// helper function to reduce code duplication when generating almost identical observers for the hover text color change effect
fn text_color_on_hover<T: Debug + Clone + Reflect>(
    color: Color,
) -> impl FnMut(On<Pointer<T>>, Query<&mut TextColor, With<ContextMenuItemText>>, Query<&Children>)
{
    move |mut event: On<Pointer<T>>,
          mut text_color: Query<&mut TextColor, With<ContextMenuItemText>>,
          children: Query<&Children>| {
        let Ok(children) = children.get(event.original_event_target()) else {
            return;
        };
        event.propagate(false);

        // find the text among children and change its color
        for child in children.iter() {
            if let Ok(mut col) = text_color.get_mut(child) {
                col.0 = color;
            }
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn_scene(bsn! {
        background()
        on(|event: On<Pointer<Press>>, query: Query<(), With<ContextMenu>>, mut commands: Commands| {
            debug!("click: {}", event.pointer_location.position);

            if query.is_empty() {
                // Open the context menu at the pointer location if one does not exist
                commands.trigger(OpenContextMenu {
                    pos: event.pointer_location.position,
                });
            } else {
                // Close the context menu if it exists
                commands.trigger(CloseContextMenus);
            }
        })
    });
}

fn on_trigger_close_menus(
    _event: On<CloseContextMenus>,
    mut commands: Commands,
    menus: Query<Entity, With<ContextMenu>>,
) {
    for e in menus.iter() {
        commands.entity(e).despawn();
    }
}

fn on_trigger_menu(event: On<OpenContextMenu>, mut commands: Commands) {
    commands.trigger(CloseContextMenus);

    let pos = event.pos;

    debug!("open context menu at: {pos}");

    commands.spawn_scene(bsn! {
        Name::new("context menu")
        ContextMenu
        Node {
            position_type: PositionType::Absolute,
            left: px(pos.x),
            top: px(pos.y),
            flex_direction: FlexDirection::Column,
            border_radius: BorderRadius::all(px(4)),
        }
        BorderColor::all(Color::BLACK)
        BackgroundColor(Color::linear_rgb(0.1, 0.1, 0.1))
        ListBox
        Children [
            context_item("fuchsia", basic::FUCHSIA),
            context_item("gray", basic::GRAY),
            context_item("maroon", basic::MAROON),
            context_item("purple", basic::PURPLE),
            context_item("teal", basic::TEAL),
        ]
        on(|event: On<ValueChange<Entity>>,
            menu_items: Query<&ContextMenuItem, With<ListItem>>,
            mut clear_col: ResMut<ClearColor>,
            mut commands: Commands| {
                let Ok(selected) = menu_items.get(event.value) else {
                    return;
                };
                clear_col.0 = selected.0.into();
                commands.trigger(CloseContextMenus);

                // We do not set the `Selected` state of any of the items because the menu
                // will be despawned.
        })
    });
}

fn context_item(text: &'static str, col: Srgba) -> impl Scene {
    bsn! {
        Name::new(format!("item-{text}"))
        ListItem
        ContextMenuItem(col)
        Node {
            padding: UiRect::all(px(5)),
        }
        Children [
            ContextMenuItemText
            Pickable::IGNORE
            Text::new(text)
            TextFont {
                font_size: FontSize::Px(24.0),
            }
            TextColor(Color::WHITE)
        ]
    }
}

fn background() -> impl Scene {
    bsn! {
        Name::new("background")
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        ZIndex({-10})
        Children [
            Text::new("Click anywhere to spawn a Context Menu.\nYour selection will change the background color.")
            TextFont {
                font_size: FontSize::Px(28.0),
            }
            TextColor(Color::WHITE)
        ]
    }
}
