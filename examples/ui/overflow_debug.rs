//! Tests how different transforms behave when clipped with `Overflow::Hidden`
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const CONTAINER_SIZE: f32 = 150.0;
const HALF_CONTAINER_SIZE: f32 = CONTAINER_SIZE / 2.0;
const LOOP_LENGTH: f32 = 4.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AnimationState {
            playing: false,
            paused_at: 0.0,
            paused_total: 0.0,
            t: 0.0,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_overflow,
                next_container_size,
                update_transform::<Move>,
                update_transform::<Scale>,
                update_transform::<Rotate>,
                update_animation,
            ),
        )
        .run();
}

#[derive(Resource)]
struct AnimationState {
    playing: bool,
    paused_at: f32,
    paused_total: f32,
    t: f32,
}

#[derive(Component)]
struct Container(u8);

trait UpdateTransform {
    fn update(&self, t: f32, transform: &mut Transform);
}

#[derive(Component)]
struct Move;

impl UpdateTransform for Move {
    fn update(&self, t: f32, transform: &mut Transform) {
        transform.translation.x = (t * TAU - FRAC_PI_2).sin() * HALF_CONTAINER_SIZE;
        transform.translation.y = -(t * TAU - FRAC_PI_2).cos() * HALF_CONTAINER_SIZE;
    }
}

#[derive(Component)]
struct Scale;

impl UpdateTransform for Scale {
    fn update(&self, t: f32, transform: &mut Transform) {
        transform.scale.x = 1.0 + 0.5 * (t * TAU).cos().max(0.0);
        transform.scale.y = 1.0 + 0.5 * (t * TAU + PI).cos().max(0.0);
    }
}

#[derive(Component)]
struct Rotate;

impl UpdateTransform for Rotate {
    fn update(&self, t: f32, transform: &mut Transform) {
        transform.rotation = Quat::from_axis_angle(Vec3::Z, ((t * TAU).cos() * 45.0).to_radians());
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        height: Val::Px(32.),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    background_color: Color::DARK_GRAY.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        [
                            "Toggle Overflow (O)",
                            "Next Container Size (S)",
                            "Toggle Animation (space)",
                        ]
                        .join("  ·  "),
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 18.0,
                            color: Color::WHITE,
                        },
                    ));
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_grow: 1.,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    spawn_row(parent, |parent| {
                        spawn_image(parent, &asset_server, Move);
                        spawn_image(parent, &asset_server, Scale);
                        spawn_image(parent, &asset_server, Rotate);
                    });

                    spawn_row(parent, |parent| {
                        spawn_text(parent, &asset_server, Move);
                        spawn_text(parent, &asset_server, Scale);
                        spawn_text(parent, &asset_server, Rotate);
                    });
                });
        });
}

fn spawn_row(parent: &mut ChildBuilder, spawn_children: impl FnOnce(&mut ChildBuilder)) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(50.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                ..default()
            },
            ..default()
        })
        .with_children(spawn_children);
}

fn spawn_image(
    parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn(ImageBundle {
            image: UiImage::new(asset_server.load("branding/bevy_logo_dark_big.png")),
            style: Style {
                height: Val::Px(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(-50.),
                left: Val::Px(-200.),
                ..default()
            },
            ..default()
        });
    });
}

fn spawn_text(
    parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn(TextBundle::from_section(
            "Bevy",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 120.0,
                color: Color::WHITE,
            },
        ));
    });
}

fn spawn_container(
    parent: &mut ChildBuilder,
    update_transform: impl UpdateTransform + Component,
    spawn_children: impl FnOnce(&mut ChildBuilder),
) {
    let mut transform = Transform::default();

    update_transform.update(0.0, &mut transform);

    parent
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(CONTAINER_SIZE),
                    height: Val::Px(CONTAINER_SIZE),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                background_color: Color::DARK_GRAY.into(),
                ..default()
            },
            Container(0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            top: Val::Px(transform.translation.x),
                            left: Val::Px(transform.translation.y),
                            ..default()
                        },
                        transform,
                        ..default()
                    },
                    update_transform,
                ))
                .with_children(spawn_children);
        });
}

fn update_animation(
    mut animation: ResMut<AnimationState>,
    time: Res<Time>,
    keys: Res<Input<KeyCode>>,
) {
    let time = time.elapsed_seconds();

    if keys.just_pressed(KeyCode::Space) {
        animation.playing = !animation.playing;

        if !animation.playing {
            animation.paused_at = time;
        } else {
            animation.paused_total += time - animation.paused_at;
        }
    }

    if animation.playing {
        animation.t = (time - animation.paused_total) % LOOP_LENGTH / LOOP_LENGTH;
    }
}

fn update_transform<T: UpdateTransform + Component>(
    animation: Res<AnimationState>,
    mut containers: Query<(&mut Transform, &mut Style, &T)>,
) {
    for (mut transform, mut style, update_transform) in &mut containers {
        update_transform.update(animation.t, &mut transform);

        style.left = Val::Px(transform.translation.x);
        style.top = Val::Px(transform.translation.y);
    }
}

fn toggle_overflow(keys: Res<Input<KeyCode>>, mut containers: Query<&mut Style, With<Container>>) {
    if keys.just_pressed(KeyCode::O) {
        for mut style in &mut containers {
            style.overflow = match style.overflow {
                Overflow {
                    x: OverflowAxis::Visible,
                    y: OverflowAxis::Visible,
                } => Overflow::clip_y(),
                Overflow {
                    x: OverflowAxis::Visible,
                    y: OverflowAxis::Clip,
                } => Overflow::clip_x(),
                Overflow {
                    x: OverflowAxis::Clip,
                    y: OverflowAxis::Visible,
                } => Overflow::clip(),
                _ => Overflow::visible(),
            };
        }
    }
}

fn next_container_size(
    keys: Res<Input<KeyCode>>,
    mut containers: Query<(&mut Style, &mut Container)>,
) {
    if keys.just_pressed(KeyCode::S) {
        for (mut style, mut container) in &mut containers {
            container.0 = (container.0 + 1) % 3;

            style.width = match container.0 {
                2 => Val::Px(30.),
                _ => Val::Px(CONTAINER_SIZE),
            };
            style.height = match container.0 {
                1 => Val::Px(30.),
                _ => Val::Px(CONTAINER_SIZE),
            };
        }
    }
}
