//! Node can choose Camera as the layout or [`UiContainSet`](bevy::prelude::UiContainSet) Component for layout.
//! Nodes will be laid out according to the size and Transform of `UiContainSet`

use std::{cell::OnceCell, sync::OnceLock};

use bevy::{
    app::Propagate, input::common_conditions::input_just_released, prelude::*, sprite::Anchor,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_camera,
                update_contain,
                switch_node.run_if(input_just_released(KeyCode::Space)),
            ),
        )
        .run();
}

#[derive(Component)]
struct ContainNode;

#[derive(Component)]
struct UiContain;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // world center
    commands.spawn(Sprite {
        custom_size: Some(Vec2::new(5.0, 5.0)),
        ..Default::default()
    });

    // Initialize uicontain
    let uicontain = commands
        .spawn((
            UiContainSize(Vec2::new(300.0, 300.0)),
            Anchor::TOP_LEFT,
            UiContainOverflow(Overflow::clip()),
            // Transform::from_xyz(-500.0, 0.0, 0.0),
            // Sprite {
            //     custom_size: Some(Vec2::new(300.0, 300.0)),
            //     ..Default::default()
            // },
            UiContain,
        ))
        .id();

    commands
        .spawn((
            Node {
                display: Display::Block,
                width: px(300.0),
                height: px(300.0),
                border: px(4.0).all(),
                // overflow:Overflow::clip(),
                ..Default::default()
            },
            BorderColor {
                top: Srgba::BLUE.into(),
                right: Srgba::GREEN.into(),
                bottom: Srgba::RED.into(),
                left: Srgba::WHITE.into(),
            },
            Propagate(UiContainTarget(uicontain)),
            ContainNode, // Button,
        ))
        .with_children(|parent| {
            let entity = parent
                .spawn((
                    Node {
                        display: Display::Block,
                        width: px(700.0),
                        height: px(700.0),
                        border: px(4.0).all(),
                        ..Default::default()
                    },
                    BorderColor {
                        top: Srgba::BLUE.into(),
                        right: Srgba::GREEN.into(),
                        bottom: Srgba::RED.into(),
                        left: Srgba::WHITE.into(),
                    },
                ))
                .with_children(|parent| {
                    let entity = parent
                        .spawn((
                            Node {
                                display: Display::Block,
                                width: px(500.0),
                                height: px(500.0),
                                border: px(4.0).all(),
                                ..Default::default()
                            },
                            BorderColor {
                                top: Srgba::BLUE.with_blue(0.5).into(),
                                right: Srgba::GREEN.with_green(0.5).into(),
                                bottom: Srgba::RED.with_red(0.5).into(),
                                left: Srgba::WHITE.into(),
                            },
                        ))
                        .with_child(ImageNode::new(
                            asset_server.load("branding/bevy_bird_dark.png"),
                        ));
                });
        });
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

fn update_contain(
    query: Query<&mut Transform, With<UiContainSize>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for mut trans in query {
        let left = input.pressed(KeyCode::KeyA) as i8 as f32;
        let right = input.pressed(KeyCode::KeyD) as i8 as f32;
        let up = input.pressed(KeyCode::KeyW) as i8 as f32;
        let down = input.pressed(KeyCode::KeyS) as i8 as f32;

        trans.translation.x += right - left;
        trans.translation.y += up - down;
    }
}

fn switch_node(
    mut commands: Commands,
    query: Single<(Entity, Has<UiContainTarget>), With<ContainNode>>,
    contain: Single<Entity, With<UiContain>>,
) {
    let (entity_node, is_contain_node) = query.into_inner();

    if is_contain_node {
        commands
            .entity(entity_node)
            .remove::<Propagate<UiContainTarget>>();
        info!("此处运行1");
    } else {
        commands
            .entity(entity_node)
            .insert(Propagate(UiContainTarget(contain.into_inner())));
        info!("此处运行2");
    }
}
