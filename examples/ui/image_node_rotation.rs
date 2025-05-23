//! Shows usage of `ImageNode::rotation`

#[cfg(feature = "bevy_ui_debug")]
use bevy::prelude::UiDebugOptions;
use bevy::{
    app::{App, Startup, Update},
    color::Color,
    core_pipeline::core_2d::Camera2d,
    ecs::{
        children,
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut, Single},
    },
    input::{keyboard::KeyCode, ButtonInput},
    math::ops,
    prelude::SpawnRelated,
    state::{
        app::AppExtStates,
        state::{NextState, OnEnter, OnExit, State, States},
    },
    time::Time,
    ui::{widget::ImageNode, FlexDirection, JustifyContent, Node, PositionType, Val},
    DefaultPlugins,
};
use bevy_asset::AssetServer;

const IMAGE_NODE_SIZE: f32 = 128.;

fn main() {
    let mut app = App::new();

    #[cfg(feature = "bevy_ui_debug")]
    app.insert_resource(UiDebugOptions {
        enabled: true,
        show_clipped: true,
        show_hidden: true,
        line_width: 3.,
    });

    app.add_plugins(DefaultPlugins)
        .init_state::<Mode>()
        .add_systems(Startup, setup)
        .add_systems(Update, (switch_modes, rotate_image_nodes))
        .add_systems(OnEnter(Mode::Image), image_mode)
        .add_systems(OnExit(Mode::Image), drop_previos_ui)
        .add_systems(OnEnter(Mode::Slices), image_mode)
        .add_systems(OnExit(Mode::Slices), drop_previos_ui)
        .run();
}

#[derive(Component)]
struct UiMarker;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
enum Mode {
    #[default]
    Image,
    Slices,
}

impl Mode {
    fn next(&self) -> Self {
        match self {
            Self::Image => Self::Slices,
            Self::Slices => Self::Image,
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn switch_modes(
    keys: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<Mode>>,
    mut next_state: ResMut<NextState<Mode>>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        next_state.set(dbg!(current_state.next()));
    }
}

fn rotate_image_nodes(mut image_nodes: Query<&mut ImageNode>, time: Res<Time>) {
    for mut node in image_nodes.iter_mut() {
        node.rotation = ops::sin(time.elapsed_secs());
    }
}

fn drop_previos_ui(mut commands: Commands, ui: Single<Entity, With<UiMarker>>) {
    commands.entity(*ui).despawn();
}

fn image_mode(mut commands: Commands, asset_server: Res<AssetServer>) {
    let branding = asset_server.load("branding/icon.png");
    commands.spawn((
        UiMarker,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            ..Default::default()
        },
        children![(
            Node {
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            children![
                (
                    Node {
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                    children![
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                ..Default::default()
                            }
                        )
                    ]
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Row,
                        ..Default::default()
                    },
                    children![
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                flip_y: true,
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                flip_y: true,
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                flip_y: true,
                                ..Default::default()
                            }
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            ImageNode {
                                color: Color::WHITE,
                                image: branding.clone(),
                                flip_y: true,
                                ..Default::default()
                            }
                        )
                    ]
                )
            ]
        )],
    ));
}
