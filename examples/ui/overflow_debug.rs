//! Tests how different transforms behave when clipped with `Overflow::Hidden`
use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const CONTAINER_SIZE: f32 = 150.0;
const HALF_CONTAINER_SIZE: f32 = CONTAINER_SIZE / 2.0;
const LOOP_LENGTH: f32 = 4.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<AnimationState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                toggle_overflow.run_if(input_just_pressed(KeyCode::KeyO)),
                next_container_size.run_if(input_just_pressed(KeyCode::KeyS)),
                update_transform::<Move>,
                update_transform::<Scale>,
                update_transform::<Rotate>,
                update_animation,
            ),
        )
        .run();
}

#[derive(Component)]
struct Instructions;

#[derive(Resource, Default)]
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

    // Instructions

    let text_style = TextStyle::default();

    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Next Overflow Setting (O)\nNext Container Size (S)\nToggle Animation (space)\n\n",
                text_style.clone(),
            ),
            TextSection::new(format!("{:?}", Overflow::clip()), text_style.clone()),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
        Instructions,
    ));

    // Overflow Debug

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        grid_template_columns: RepeatedGridTrack::px(3, CONTAINER_SIZE),
                        grid_template_rows: RepeatedGridTrack::px(2, CONTAINER_SIZE),
                        row_gap: Val::Px(80.),
                        column_gap: Val::Px(80.),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    spawn_image(parent, &asset_server, Move);
                    spawn_image(parent, &asset_server, Scale);
                    spawn_image(parent, &asset_server, Rotate);

                    spawn_text(parent, &asset_server, Move);
                    spawn_text(parent, &asset_server, Scale);
                    spawn_text(parent, &asset_server, Rotate);
                });
        });
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
                ..default()
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
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    overflow: Overflow::clip(),
                    ..default()
                },
                background_color: Color::srgb(0.25, 0.25, 0.25).into(),
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
    keys: Res<ButtonInput<KeyCode>>,
) {
    let delta = time.elapsed_seconds();

    if keys.just_pressed(KeyCode::Space) {
        animation.playing = !animation.playing;

        if !animation.playing {
            animation.paused_at = delta;
        } else {
            animation.paused_total += delta - animation.paused_at;
        }
    }

    if animation.playing {
        animation.t = (delta - animation.paused_total) % LOOP_LENGTH / LOOP_LENGTH;
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

fn toggle_overflow(
    mut containers: Query<&mut Style, With<Container>>,
    mut instructions: Query<&mut Text, With<Instructions>>,
) {
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

        let mut text = instructions.single_mut();
        text.sections[1].value = format!("{:?}", style.overflow);
    }
}

fn next_container_size(mut containers: Query<(&mut Style, &mut Container)>) {
    for (mut style, mut container) in &mut containers {
        container.0 = (container.0 + 1) % 3;

        style.width = match container.0 {
            2 => Val::Percent(30.),
            _ => Val::Percent(100.),
        };
        style.height = match container.0 {
            1 => Val::Percent(30.),
            _ => Val::Percent(100.),
        };
    }
}
