//! This example illustrates how to create a context menu that changes the clear color

use bevy::{color::palettes::basic, prelude::*};
use bevy_ecs::{relationship::RelatedSpawner, spawn::SpawnWith};

#[derive(Event)]
struct OpenContextMenu {
    pos: Vec2,
}

#[derive(Event)]
struct CloseContextMenus;

#[derive(Component)]
struct ContextMenu;

#[derive(Component)]
struct ContextMenuItem(Srgba);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(on_trigger_menu)
        .add_observer(on_trigger_close_menus)
        .add_observer(
            |trigger: Trigger<Pointer<Over>>,
             mut query: Query<&mut TextColor>,
             children: Query<&Children>| {
                let Ok(children) = children.get(trigger.target()) else {
                    return Ok(());
                };

                for child in children.iter() {
                    if let Ok(mut color) = query.get_mut(child) {
                        color.0 = basic::RED.into();
                    }
                }

                Ok(())
            },
        )
        .add_observer(
            |trigger: Trigger<Pointer<Out>>,
             mut query: Query<&mut TextColor>,
             children: Query<&Children>| {
                let Ok(children) = children.get(trigger.target()) else {
                    return Ok(());
                };

                for child in children.iter() {
                    if let Ok(mut color) = query.get_mut(child) {
                        color.0 = basic::WHITE.into();
                    }
                }

                Ok(())
            },
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn(button()).observe(
        // any click bubbling up here should lead to closing any open menu
        |_trigger: Trigger<Pointer<Pressed>>, mut commands: Commands| {
            commands.trigger(CloseContextMenus);
        },
    );
}

fn on_trigger_close_menus(
    _trigger: Trigger<CloseContextMenus>,
    mut commands: Commands,
    menus: Query<Entity, With<ContextMenu>>,
) {
    for e in menus.iter() {
        commands.entity(e).despawn();
    }
}

fn on_trigger_menu(trigger: Trigger<OpenContextMenu>, mut commands: Commands) {
    commands.trigger(CloseContextMenus);

    let pos = trigger.pos;

    debug!("open context menu at: {pos}");

    commands
        .spawn((
            Name::new("context menu"),
            ContextMenu,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(pos.x),
                top: Val::Px(pos.y),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BorderColor(Color::BLACK),
            BorderRadius::all(Val::Px(4.)),
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
            |trigger: Trigger<Pointer<Pressed>>,
             query: Query<&ContextMenuItem>,
             mut clear_col: ResMut<ClearColor>,
             mut commands: Commands| {
                let target = trigger.event().target;

                if let Ok(item) = query.get(target) {
                    clear_col.0 = item.0.into();
                    commands.trigger(CloseContextMenus);
                }
            },
        );
}

fn context_item(text: &str, col: Srgba) -> impl Bundle + use<> {
    (
        Name::new(format!("item-{}", text)),
        ContextMenuItem(col),
        Button,
        Node {
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(5.0)),
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

fn button() -> impl Bundle + use<> {
    (
        Name::new("button"),
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        ZIndex(-10),
        Children::spawn(SpawnWith(|parent: &mut RelatedSpawner<ChildOf>| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(250.0),
                        height: Val::Px(65.0),
                        border: UiRect::all(Val::Px(5.0)),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BorderColor(Color::BLACK),
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
                .observe(
                    |mut trigger: Trigger<Pointer<Pressed>>, mut commands: Commands| {
                        trigger.propagate(false);

                        debug!("click: {}", trigger.pointer_location.position);

                        commands.trigger(OpenContextMenu {
                            pos: trigger.pointer_location.position,
                        });
                    },
                );
        })),
    )
}
