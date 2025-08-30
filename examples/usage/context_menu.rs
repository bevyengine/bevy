//! This example illustrates how to create a context menu that changes the clear color

use bevy::{
    color::palettes::basic,
    ecs::{relationship::RelatedSpawner, spawn::SpawnWith},
    prelude::*,
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
#[derive(Component)]
struct ContextMenu;

/// context menu item data storing what background color `Srgba` it activates
#[derive(Component)]
struct ContextMenuItem(Srgba);

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
) -> impl FnMut(On<Pointer<T>>, Query<&mut TextColor>, Query<&Children>) {
    move |mut event: On<Pointer<T>>,
          mut text_color: Query<&mut TextColor>,
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

    commands.spawn(background_and_button()).observe(
        // any click bubbling up here should lead to closing any open menu
        |_: On<Pointer<Press>>, mut commands: Commands| {
            commands.trigger(CloseContextMenus);
        },
    );
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

    commands
        .spawn((
            Name::new("context menu"),
            ContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: px(pos.x),
                top: px(pos.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BorderRadius::all(px(4)),
            BackgroundColor(Color::linear_rgb(0.1, 0.1, 0.1)),
            children![
                context_item("fuchsia", basic::FUCHSIA),
                context_item("gray", basic::GRAY),
                context_item("maroon", basic::MAROON),
                context_item("purple", basic::PURPLE),
                context_item("teal", basic::TEAL),
            ],
        ))
        .observe(
            |event: On<Pointer<Press>>,
             menu_items: Query<&ContextMenuItem>,
             mut clear_col: ResMut<ClearColor>,
             mut commands: Commands| {
                let target = event.original_event_target();

                if let Ok(item) = menu_items.get(target) {
                    clear_col.0 = item.0.into();
                    commands.trigger(CloseContextMenus);
                }
            },
        );
}

fn context_item(text: &str, col: Srgba) -> impl Bundle {
    (
        Name::new(format!("item-{text}")),
        ContextMenuItem(col),
        Button,
        Node {
            padding: UiRect::all(px(5)),
            ..default()
        },
        children![(
            Pickable::IGNORE,
            Text::new(text),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
        )],
    )
}

fn background_and_button() -> impl Bundle {
    (
        Name::new("background"),
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ZIndex(-10),
        Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
            parent
                .spawn((
                    Name::new("button"),
                    Button,
                    Node {
                        width: px(250),
                        height: px(65),
                        border: UiRect::all(px(5)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BorderRadius::MAX,
                    BackgroundColor(Color::BLACK),
                    children![(
                        Pickable::IGNORE,
                        Text::new("Context Menu"),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextShadow::default(),
                    )],
                ))
                .observe(|mut event: On<Pointer<Press>>, mut commands: Commands| {
                    // by default this event would bubble up further leading to the `CloseContextMenus`
                    // event being triggered and undoing the opening of one here right away.
                    event.propagate(false);

                    debug!("click: {}", event.pointer_location.position);

                    commands.trigger(OpenContextMenu {
                        pos: event.pointer_location.position,
                    });
                });
        })),
    )
}
