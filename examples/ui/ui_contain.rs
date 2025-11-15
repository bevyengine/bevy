//! Node can choose Camera as the layout or [`UiContainSet`](bevy::prelude::UiContainSet) Component for layout.
//! Nodes will be laid out according to the size and Transform of `UiContainSet`

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
                update_text,
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
struct InfoTextUiContain(Entity);

#[derive(Component)]
struct InfoTextCamera(Entity);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let camera = commands.spawn((Camera2d, CameraInfo)).id();

    // world center
    commands.spawn(Sprite {
        custom_size: Some(Vec2::new(5.0, 5.0)),
        ..Default::default()
    });

    // Initialize uicontain
    let ui_contain = commands
        .spawn((
            UiContainSize(Vec2::new(300.0, 300.0)),
            Anchor::TOP_LEFT,
            // UiContainOverflow(Overflow::clip()),
            // Transform::from_xyz(-500.0, 0.0, 0.0),
            // Sprite {
            //     custom_size: Some(Vec2::new(300.0, 300.0)),
            //     ..Default::default()
            // },
            UiContainInfo,
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
            Propagate(UiContainTarget(ui_contain)),
            ContainNode, // Button,
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
        Text::new("WASD move uicontain\nArrowKey move camera\nSpace Switching node type"),
        GlobalZIndex(10),
        TextColor(Srgba::rgb(0.0, 1.0, 1.0).into()),
    ));

    commands
        .spawn((Node {
            position_type: PositionType::Absolute,
            bottom: px(0.0),
            right: px(0.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GlobalTranfrom Camera: None"),
                InfoTextCamera(camera),
            ));
            parent.spawn((
                Text::new("GlobalTranfrom UiContain: None"),
                InfoTextUiContain(ui_contain),
            ));
        });
}

fn update_text(
    info_camera: Single<(&mut Text, &InfoTextCamera), Without<InfoTextUiContain>>,
    info_uicontain: Single<(&mut Text, &InfoTextUiContain), Without<InfoTextCamera>>,
    query: Query<&GlobalTransform>,
) {
    let (mut text_camera, related) = info_camera.into_inner();

    text_camera.0 = format!(
        "GlobalTranfrom Camera: {:?}",
        query.get(related.0).map(|trans| trans.translation()),
    );

    let (mut text_contain, related) = info_uicontain.into_inner();
    text_contain.0 = format!(
        "GlobalTranfrom UiContain: {:?}",
        query.get(related.0).map(|trans| trans.translation()),
    );
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

fn switch_node(
    mut commands: Commands,
    query: Single<(Entity, Has<UiContainTarget>), With<ContainNode>>,
    contain: Single<Entity, With<UiContainInfo>>,
) {
    let (entity_node, is_contain_node) = query.into_inner();

    if is_contain_node {
        commands
            .entity(entity_node)
            .remove::<Propagate<UiContainTarget>>();

    } else {
        commands
            .entity(entity_node)
            .insert(Propagate(UiContainTarget(contain.into_inner())));
    }
}
