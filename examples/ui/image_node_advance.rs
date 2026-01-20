//! This example illustrates the advance usage of an image node.

use bevy::{color::palettes::tailwind, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // enable for image outline
        .insert_resource(UiDebugOptions {
            enabled: true,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        // observer for enlarge ImageGroup height
        .add_observer(
            |_: On<ImageGroupEnlarge>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.height {
                        let new_val = (val + 1.).min(50.0);
                        node.height = Val::Percent(new_val);
                    }
                }
            },
        )
        // observer for earrow ImageGroup height
        .add_observer(
            |_: On<ImageGroupNarrow>, query: Query<&mut Node, With<ImageGroup>>| {
                for mut node in query {
                    if let Val::Percent(val) = node.height {
                        let new_val = (val - 1.).max(10.);
                        node.height = Val::Percent(new_val);
                    }
                }
            },
        )
        .run();
}

#[derive(Debug, Component)]
struct ImageGroup;

#[derive(Debug, Event)]
struct ImageGroupEnlarge;

#[derive(Debug, Event)]
struct ImageGroupNarrow;

#[derive(Debug, Component)]
struct TextMeta {
    height: f32,
}

#[derive(Debug, EntityEvent)]
struct TextEvent {
    entity: Entity,
    change: f32,
}

// press `h/H` and `↑`/`↓` to resize height
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image_handle = asset_server.load("branding/icon.png");
    commands.spawn(Camera2d);
    // Keyboard Hint
    commands
        .spawn((
            TextMeta { height: 40.0 },
            Text::new("press `h/H` and `↑`/`↓` to resize height\nheight : 10%"),
            TextColor::WHITE,
            Node {
                position_type: PositionType::Absolute,
                top: px(4),
                left: px(4),
                ..default()
            },
        ))
        .observe(update_text);

    commands
        .spawn((Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceAround,
            width: percent(100),
            height: percent(100),
            padding: UiRect::all(Val::Px(10.)),
            ..default()
        },))
        .with_children(|builder| {
            // `NodeImageMode::Auto` will maintain the original image's aspect ratio when possible
            builder.spawn((
                ImageGroup,
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::Start,
                    width: Val::Percent(100.),
                    height: Val::Percent(40.),
                    ..default()
                },
                BackgroundColor(Color::from(tailwind::BLUE_100)),
                children![
                    (ImageNode {
                        image: image_handle.clone(),
                        image_mode: NodeImageMode::Auto,
                        ..default()
                    },),
                    (ImageNode {
                        image: image_handle.clone(),
                        image_mode: NodeImageMode::Auto,
                        ..default()
                    },),
                    (ImageNode {
                        image: image_handle.clone(),
                        image_mode: NodeImageMode::Auto,
                        ..default()
                    },),
                    (ImageNode {
                        image: image_handle.clone(),
                        image_mode: NodeImageMode::Auto,
                        ..default()
                    },)
                ],
            ));
            // `NodeImageMode::Stretch` will resized to match the size of the `Node` component
            builder.spawn((
                ImageGroup,
                Node {
                    display: Display::Flex,
                    justify_content: JustifyContent::Start,
                    width: Val::Percent(100.),
                    height: Val::Percent(40.),
                    ..default()
                },
                BackgroundColor(Color::from(tailwind::BLUE_100)),
                children![
                    (
                        Node {
                            height: Val::Percent(100.),
                            width: Val::Percent(10.),
                            ..default()
                        },
                        ImageNode {
                            image: image_handle.clone(),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ),
                    (
                        Node {
                            height: Val::Percent(100.),
                            width: Val::Percent(20.),
                            ..default()
                        },
                        ImageNode {
                            image: image_handle.clone(),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ),
                    (
                        Node {
                            height: Val::Percent(100.),
                            width: Val::Percent(30.),
                            ..default()
                        },
                        ImageNode {
                            image: image_handle.clone(),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    ),
                    (
                        Node {
                            height: Val::Percent(100.),
                            width: Val::Percent(40.),
                            ..default()
                        },
                        ImageNode {
                            image: image_handle.clone(),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                    )
                ],
            ));
        });
}

// trigger event
fn update(
    keycode: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    query: Query<Entity, With<TextMeta>>,
) {
    let entity = query.single().unwrap();
    if keycode.pressed(KeyCode::ArrowUp) {
        commands.trigger(ImageGroupEnlarge);
        commands.trigger(TextEvent {
            entity,
            change: 1.0,
        });
    }
    if keycode.pressed(KeyCode::ArrowDown) {
        commands.trigger(ImageGroupNarrow);
        commands.trigger(TextEvent {
            entity,
            change: -1.,
        });
    }
}

fn update_text(
    event: On<TextEvent>,
    mut textmeta: Single<&mut TextMeta>,
    mut text: Single<&mut Text>,
) {
    let str = "press `h/H` and `↑`/`↓` to resize height\n";
    let mut new_text = Text::new(str);
    textmeta.height = (textmeta.height + event.change).clamp(10.0, 50.0);
    new_text.push_str(&format!("height : {}%", textmeta.height));
    text.0 = new_text.0;
}
