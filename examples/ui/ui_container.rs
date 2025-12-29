//! Nodes will be laid out based on the container's size [`UiContainerSize`](UiContainerSize)
//! and their position in world space (affected by the container's Transform).

use bevy::{
    app::Propagate,
    input::common_conditions::{input_just_pressed, input_just_released},
    prelude::*,
    sprite::Anchor,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_camera_pos,
                update_contain_pos,
                switch_node_type.run_if(input_just_released(KeyCode::Space)),
                switch_ui_contain_overflow.run_if(input_just_released(KeyCode::Digit1)),
                switch_ui_contain_anchor.run_if(input_just_released(KeyCode::Digit2)),
                update_text,
                click_button.run_if(input_just_pressed(MouseButton::Left)),
            ),
        )
        .run();
}

#[derive(Component)]
struct ContainNode;

#[derive(Component)]
struct UiContainInfo;

#[derive(Component)]
struct CameraInfo;

#[derive(Component)]
struct InfoText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((Camera2d, CameraInfo));

    // world center
    commands.spawn(Sprite {
        custom_size: Some(Vec2::new(5.0, 5.0)),
        ..Default::default()
    });

    // Initialize uicontainer
    let ui_contain = commands
        .spawn((
            UiContainer(UVec2::new(300, 300)),
            Anchor::TOP_LEFT,
            UiContainerOverflow::default(),
            UiContainInfo,
        ))
        .id();

    commands
        .spawn((
            Node {
                display: Display::Block,
                width: percent(100.0),
                height: percent(100.0),
                border: px(4.0).all(),
                ..Default::default()
            },
            BorderColor {
                top: Srgba::BLUE.into(),
                right: Srgba::GREEN.into(),
                bottom: Srgba::RED.into(),
                left: Srgba::WHITE.into(),
            },
            Propagate(UiContainerTarget(ui_contain)),
            ContainNode,
            Button,
        ))
        .with_children(|parent| {
            parent
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
                    parent
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

    // text spawn
    commands.spawn((
        Node {
            display: Display::Grid,
            position_type: PositionType::Absolute,
            bottom: px(0.0),
            ..Default::default()
        },
        Text::new(
            "WASD move uicontain
ArrowKey move camera
Space Switching node type
Digit1 Switching uicontain Overflow
Digit2 Switching uicontain Anchor
Click RootNode to change backgroundcolor",
        ),
        GlobalZIndex(10),
        TextColor(Srgba::rgb(0.0, 1.0, 1.0).into()),
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(0.0),
                right: px(0.0),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(
                    "UiContain: 
Anchor: None
Overflow: UnKnown
GlobalTransform: None
GlobalTransform Camera: None",
                ),
                InfoText,
            ));
        });
}

fn update_text(
    info_camera: Single<Entity, With<Camera>>,
    info_uicontain: Single<(Entity, &Anchor, &UiContainerOverflow), With<UiContainInfo>>,
    info_text: Single<&mut Text, With<InfoText>>,
    query: Query<&GlobalTransform>,
) {
    let mut info_text = info_text.into_inner();

    let (entity_contain, anchor, overflow) = info_uicontain.into_inner();

    let overflow_text = if overflow.0 == Overflow::visible() {
        "Overflow::visible"
    } else if overflow.0 == Overflow::clip() {
        "Overflow::clip"
    } else if overflow.0 == Overflow::clip_x() {
        "Overflow::clip_x"
    } else if overflow.0 == Overflow::clip_y() {
        "Overflow::clip_y"
    } else {
        "UnKnown"
    };

    let global_camera = query
        .get(info_camera.into_inner())
        .map(GlobalTransform::translation)
        .ok();
    let global_uicontain = query
        .get(entity_contain)
        .map(GlobalTransform::translation)
        .ok();

    info_text.0 = format!(
        "UiContain: 
Anchor: {:?}
Overflow: {:?}
GlobalTransform: {:?}
GlobalTransform Camera: {:?}",
        anchor, overflow_text, global_uicontain, global_camera,
    );
}

fn update_camera_pos(query: Query<&mut Transform, With<Camera>>, input: Res<ButtonInput<KeyCode>>) {
    for mut trans in query {
        let left = input.pressed(KeyCode::ArrowLeft) as i8 as f32;
        let right = input.pressed(KeyCode::ArrowRight) as i8 as f32;
        let up = input.pressed(KeyCode::ArrowUp) as i8 as f32;
        let down = input.pressed(KeyCode::ArrowDown) as i8 as f32;

        trans.translation.x += right - left;
        trans.translation.y += up - down;
    }
}

fn update_contain_pos(
    query: Single<&mut Transform, With<UiContainInfo>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    let mut trans = query.into_inner();
    let left = input.pressed(KeyCode::KeyA) as i8 as f32;
    let right = input.pressed(KeyCode::KeyD) as i8 as f32;
    let up = input.pressed(KeyCode::KeyW) as i8 as f32;
    let down = input.pressed(KeyCode::KeyS) as i8 as f32;

    trans.translation.x += right - left;
    trans.translation.y += up - down;
}

fn switch_node_type(
    mut commands: Commands,
    query: Single<(Entity, Has<UiContainerTarget>), With<ContainNode>>,
    contain: Single<Entity, With<UiContainInfo>>,
) {
    let (entity_node, is_contain_node) = query.into_inner();

    if is_contain_node {
        commands
            .entity(entity_node)
            .remove::<Propagate<UiContainerTarget>>();
    } else {
        commands
            .entity(entity_node)
            .insert(Propagate(UiContainerTarget(contain.into_inner())));
    }
}

fn switch_ui_contain_overflow(query: Single<&mut UiContainerOverflow, With<UiContainInfo>>) {
    let mut overflow = query.into_inner();

    if overflow.0 == Overflow::visible() {
        overflow.0 = Overflow::clip();
    } else if overflow.0 == Overflow::clip() {
        overflow.0 = Overflow::clip_x();
    } else if overflow.0 == Overflow::clip_x() {
        overflow.0 = Overflow::clip_y();
    } else {
        overflow.0 = Overflow::visible();
    }
}

fn switch_ui_contain_anchor(query: Single<&mut Anchor, With<UiContainInfo>>) {
    let mut anchor = query.into_inner();

    match anchor.as_mut() {
        anchor if *anchor == Anchor::BOTTOM_LEFT => *anchor = Anchor::BOTTOM_CENTER,
        anchor if *anchor == Anchor::BOTTOM_CENTER => *anchor = Anchor::BOTTOM_RIGHT,
        anchor if *anchor == Anchor::BOTTOM_RIGHT => *anchor = Anchor::CENTER_LEFT,
        anchor if *anchor == Anchor::CENTER_LEFT => *anchor = Anchor::CENTER,
        anchor if *anchor == Anchor::CENTER => *anchor = Anchor::CENTER_RIGHT,
        anchor if *anchor == Anchor::CENTER_RIGHT => *anchor = Anchor::TOP_LEFT,
        anchor if *anchor == Anchor::TOP_LEFT => *anchor = Anchor::TOP_CENTER,
        anchor if *anchor == Anchor::TOP_CENTER => *anchor = Anchor::TOP_RIGHT,
        anchor if *anchor == Anchor::TOP_RIGHT => *anchor = Anchor::BOTTOM_LEFT,
        _ => *anchor = Anchor::CENTER,
    }
}

fn click_button(button: Single<(&Interaction, &mut BackgroundColor), With<ContainNode>>) {
    let (button, mut background) = button.into_inner();
    if Interaction::Pressed == *button {
        let color = background.0.to_srgba();
        if color == Srgba::NONE {
            background.0 = Srgba::RED.into();
        } else if color == Srgba::RED {
            background.0 = Srgba::BLUE.into();
        } else if color == Srgba::BLUE {
            background.0 = Srgba::GREEN.into();
        } else {
            background.0 = Srgba::NONE.into();
        }
    }
}
