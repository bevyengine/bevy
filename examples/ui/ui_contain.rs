//! Node can choose Camera as the layout or UiContact Component for layout.
//! Nodes will be laid out according to the size and Transform of UiContact

use bevy::{app::Propagate, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_camera, update_node))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let uicontain = commands
        .spawn((UiContainSet {
            scale_factor: 1.0,
            physical_size: UVec2::new(600, 600),
        },))
        .id();

    commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                right: px(0.0),
                border: px(4.0).all(),
                ..Default::default()
            },
            BorderColor {
                top: Srgba::BLUE.into(),
                right: Srgba::GREEN.into(),
                bottom: Srgba::RED.into(),
                left: Srgba::WHITE.into(),
            },
            Propagate(UiContainTarget(uicontain)),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(150.0),
                        height: px(150.0),
                        border: px(4.0).all(),
                        justify_self: JustifySelf::Center,
                        ..Default::default()
                    },
                    BorderColor {
                        top: Srgba::BLUE.into(),
                        right: Srgba::GREEN.into(),
                        bottom: Srgba::RED.into(),
                        left: Srgba::WHITE.into(),
                    },
                ))
                .with_child((Text::new("node text"),));
        });

    commands.spawn((
        Node {
            width: px(150.0),
            height: px(150.0),
            border: px(4.0).all(),
            ..Default::default()
        },
        BorderColor {
            top: Srgba::BLUE.into(),
            right: Srgba::GREEN.into(),
            bottom: Srgba::RED.into(),
            left: Srgba::WHITE.into(),
        },
    ));

    commands.spawn((
        Text2d::new("sprite"),
        TextColor(Srgba::RED.into()),
        Sprite {
            custom_size: Some(Vec2::splat(50.0)),
            ..Default::default()
        },
        Transform::from_xyz(-100.0, 0.0, 0.0),
    ));
}

fn update_camera(query: Query<&mut Transform, With<Camera>>, input: Res<ButtonInput<KeyCode>>) {
    for mut trans in query {
        let left = input.pressed(KeyCode::ArrowLeft) as i8 as f32;
        let right = input.pressed(KeyCode::ArrowRight) as i8 as f32;
        let up = input.pressed(KeyCode::ArrowUp) as i8 as f32;
        let down = input.pressed(KeyCode::ArrowDown) as i8 as f32;

        trans.translation.x += right - left;
        trans.translation.y += up - down;
    }
}

fn update_node(query: Query<&mut Transform, With<UiContainSet>>, input: Res<ButtonInput<KeyCode>>) {
    for mut trans in query {
        let left = input.pressed(KeyCode::KeyA) as i8 as f32;
        let right = input.pressed(KeyCode::KeyD) as i8 as f32;
        let up = input.pressed(KeyCode::KeyW) as i8 as f32;
        let down = input.pressed(KeyCode::KeyS) as i8 as f32;

        trans.translation.x += right - left;
        trans.translation.y += up - down;
    }
}
