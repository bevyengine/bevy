//! Tests how different transforms behave when clipped with `Overflow::Hidden`

use bevy::{input::common_conditions::input_just_pressed, prelude::*, ui::widget::TextUiWriter};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

const CONTAINER_SIZE: f32 = 150.0;
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
    fn update(&self, t: f32, transform: &mut UiTransform);
}

#[derive(Component)]
struct Move;

impl UpdateTransform for Move {
    fn update(&self, t: f32, transform: &mut UiTransform) {
        transform.translation.x = Val::Percent(ops::sin(t * TAU - FRAC_PI_2) * 50.);
        transform.translation.y = Val::Percent(-ops::cos(t * TAU - FRAC_PI_2) * 50.);
    }
}

#[derive(Component)]
struct Scale;

impl UpdateTransform for Scale {
    fn update(&self, t: f32, transform: &mut UiTransform) {
        transform.scale.x = 1.0 + 0.5 * ops::cos(t * TAU).max(0.0);
        transform.scale.y = 1.0 + 0.5 * ops::cos(t * TAU + PI).max(0.0);
    }
}

#[derive(Component)]
struct Rotate;

impl UpdateTransform for Rotate {
    fn update(&self, t: f32, transform: &mut UiTransform) {
        transform.rotation = Rot2::radians(ops::cos(t * TAU) * 45.0);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera

    commands.spawn(Camera2d);

    // Instructions

    let text_font = TextFont::default();

    commands
        .spawn((
            Text::new(
                "Next Overflow Setting (O)\nNext Container Size (S)\nToggle Animation (space)\n\n",
            ),
            text_font.clone(),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
            Instructions,
        ))
        .with_child((
            TextSpan::new(format!("{:?}", Overflow::clip())),
            text_font.clone(),
        ));

    // Overflow Debug

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Node {
                    display: Display::Grid,
                    grid_template_columns: RepeatedGridTrack::px(3, CONTAINER_SIZE),
                    grid_template_rows: RepeatedGridTrack::px(2, CONTAINER_SIZE),
                    row_gap: Val::Px(80.),
                    column_gap: Val::Px(80.),
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
    parent: &mut ChildSpawnerCommands,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn((
            ImageNode::new(asset_server.load("branding/bevy_logo_dark_big.png")),
            Node {
                height: Val::Px(100.),
                position_type: PositionType::Absolute,
                top: Val::Px(-50.),
                left: Val::Px(-200.),
                ..default()
            },
        ));
    });
}

fn spawn_text(
    parent: &mut ChildSpawnerCommands,
    asset_server: &Res<AssetServer>,
    update_transform: impl UpdateTransform + Component,
) {
    spawn_container(parent, update_transform, |parent| {
        parent.spawn((
            Text::new("Bevy"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0,
                ..default()
            },
        ));
    });
}

fn spawn_container(
    parent: &mut ChildSpawnerCommands,
    update_transform: impl UpdateTransform + Component,
    spawn_children: impl FnOnce(&mut ChildSpawnerCommands),
) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
            Container(0),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
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
    let delta = time.elapsed_secs();

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
    mut containers: Query<(&mut UiTransform, &T)>,
) {
    for (mut transform, update_transform) in &mut containers {
        update_transform.update(animation.t, &mut transform);
    }
}

fn toggle_overflow(
    mut containers: Query<&mut Node, With<Container>>,
    instructions: Single<Entity, With<Instructions>>,
    mut writer: TextUiWriter,
) {
    for mut node in &mut containers {
        node.overflow = match node.overflow {
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

        let entity = *instructions;
        *writer.text(entity, 1) = format!("{:?}", node.overflow);
    }
}

fn next_container_size(mut containers: Query<(&mut Node, &mut Container)>) {
    for (mut node, mut container) in &mut containers {
        container.0 = (container.0 + 1) % 3;

        node.width = match container.0 {
            2 => Val::Percent(30.),
            _ => Val::Percent(100.),
        };
        node.height = match container.0 {
            1 => Val::Percent(30.),
            _ => Val::Percent(100.),
        };
    }
}
