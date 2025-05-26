//! Shows usage of `ImageNode::rotation`

use bevy::{
    app::{App, Startup, Update},
    asset::AssetServer,
    color::Color,
    core_pipeline::core_2d::Camera2d,
    ecs::{
        children,
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, ResMut, Single},
    },
    image::{ImageLoaderSettings, ImageSampler},
    input::{keyboard::KeyCode, ButtonInput},
    math::ops,
    prelude::SpawnRelated,
    sprite::{BorderRect, SliceScaleMode, TextureSlicer},
    state::{
        app::AppExtStates,
        state::{NextState, OnEnter, OnExit, State, States},
    },
    time::Time,
    ui::{
        widget::{ImageNode, NodeImageMode},
        FlexDirection, JustifyContent, Node, PositionType, Val,
    },
    utils::default,
    DefaultPlugins,
};

const IMAGE_NODE_SIZE: f32 = 128.;
const UI_GAPS: f32 = 16.;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<Mode>()
        .add_systems(Startup, setup)
        .add_systems(Update, (switch_modes, rotate_image_nodes))
        .add_systems(OnEnter(Mode::Image), image_mode)
        .add_systems(OnExit(Mode::Image), drop_previous_ui)
        .add_systems(OnEnter(Mode::Slices), slices_mode)
        .add_systems(OnExit(Mode::Slices), drop_previous_ui)
        .run();
}

#[derive(Component)]
struct UiMarker;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, States)]
/// Selection of modes
enum Mode {
    /// Uses [`ImageNode`] with [`NodeImageMode::Auto`]
    #[default]
    Image,
    /// Uses [`ImageNode`] with [`NodeImageMode::Sliced`]
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

/// Spawns a camera
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// Switching between modes on the press of the Q key
fn switch_modes(
    keys: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<Mode>>,
    mut next_state: ResMut<NextState<Mode>>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        next_state.set(current_state.next());
    }
}

/// Rock the images back and forth
fn rotate_image_nodes(mut image_nodes: Query<&mut ImageNode>, time: Res<Time>) {
    for mut node in image_nodes.iter_mut() {
        node.rotation = ops::sin(time.elapsed_secs());
    }
}

/// Drops the previous UI before the creation of the new one
fn drop_previous_ui(mut commands: Commands, ui: Single<Entity, With<UiMarker>>) {
    commands.entity(*ui).despawn();
}

/// Builds the UI on entering [`Mode::Image`]
fn image_mode(mut commands: Commands, asset_server: Res<AssetServer>) {
    let branding = asset_server.load("branding/icon.png");
    build_ui(&mut commands, |flip_x| ImageNode {
        color: Color::WHITE,
        image: branding.clone(),
        flip_x,
        ..Default::default()
    });
}

/// Builds the UI on entering [`Mode::Slices`]
fn slices_mode(mut commands: Commands, asset_server: Res<AssetServer>) {
    let image = asset_server.load_with_settings(
        "textures/fantasy_ui_borders/numbered_slices.png",
        |settings: &mut ImageLoaderSettings| {
            // Need to use nearest filtering to avoid bleeding between the slices with tiling
            settings.sampler = ImageSampler::nearest();
        },
    );

    let slicer = TextureSlicer {
        // `numbered_slices.png` is 48 pixels square. `BorderRect::square(16.)` insets the slicing line from each edge by 16 pixels, resulting in nine slices that are each 16 pixels square.
        border: BorderRect::all(16.),
        // With `SliceScaleMode::Tile` the side and center slices are tiled to fill the side and center sections of the target.
        // And with a `stretch_value` of `1.` the tiles will have the same size as the corresponding slices in the source image.
        center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
        ..default()
    };

    build_ui(&mut commands, |flip_x| ImageNode {
        color: Color::WHITE,
        image: image.clone(),
        flip_x,
        image_mode: NodeImageMode::Sliced(slicer.clone()),
        ..Default::default()
    });
}

/// Builds the UI using a methods that generates the [`ImageNode`] with a parameter for flipping on Y.  
/// The UI will contain 8 [`ImageNode`] across 2 rows,
/// the 4 on the bottom row will have their Y flipped.
fn build_ui(commands: &mut Commands, image_node: impl Fn(bool) -> ImageNode) {
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
                row_gap: Val::Px(UI_GAPS),
                ..Default::default()
            },
            children![
                (
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(UI_GAPS),
                        ..Default::default()
                    },
                    children![
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(false)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(false)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(false)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(false)
                        )
                    ]
                ),
                (
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(UI_GAPS),
                        ..Default::default()
                    },
                    children![
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(true)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(true)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(true)
                        ),
                        (
                            Node {
                                width: Val::Px(IMAGE_NODE_SIZE),
                                height: Val::Px(IMAGE_NODE_SIZE),
                                ..Default::default()
                            },
                            image_node(true)
                        )
                    ]
                )
            ]
        )],
    ));
}
