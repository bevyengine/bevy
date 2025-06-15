//! An example demonstrating how to translate, rotate and scale UI elements.
use bevy::color::palettes::css::DARK_GRAY;
use bevy::color::palettes::css::RED;
use bevy::color::palettes::css::YELLOW;
use bevy::prelude::*;
use core::f32::consts::FRAC_PI_8;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .add_systems(Update, translation_system)
        .run();
}

const NORMAL_BUTTON: Color = Color::WHITE;
const HOVERED_BUTTON: Color = Color::Srgba(YELLOW);
const PRESSED_BUTTON: Color = Color::Srgba(RED);

/// A button that rotates the target node
#[derive(Component)]
pub struct RotateButton(pub Rot2);

/// A button that scales the target node
#[derive(Component)]
pub struct ScaleButton(pub f32);

/// Marker component so the systems know which entities to translate, rotate and scale
#[derive(Component)]
pub struct TargetNode;

/// Handles button interactions
fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&RotateButton>,
            Option<&ScaleButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut rotator_query: Query<&mut UiTransform, With<TargetNode>>,
) {
    for (interaction, mut color, maybe_rotate, maybe_scale) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                if let Some(step) = maybe_rotate {
                    for mut transform in rotator_query.iter_mut() {
                        transform.rotation *= step.0;
                    }
                }
                if let Some(step) = maybe_scale {
                    for mut transform in rotator_query.iter_mut() {
                        transform.scale += step.0;
                        transform.scale =
                            transform.scale.clamp(Vec2::splat(0.25), Vec2::splat(3.0));
                    }
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

// move the rotating panel when the arrow keys are pressed
fn translation_system(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut translation_query: Query<&mut UiTransform, With<TargetNode>>,
) {
    let controls = [
        (KeyCode::ArrowLeft, -Vec2::X),
        (KeyCode::ArrowRight, Vec2::X),
        (KeyCode::ArrowUp, -Vec2::Y),
        (KeyCode::ArrowDown, Vec2::Y),
    ];
    for &(key_code, direction) in &controls {
        if input.pressed(key_code) {
            for mut transform in translation_query.iter_mut() {
                let d = direction * 50.0 * time.delta_secs();
                let (Val::Px(x), Val::Px(y)) = (transform.translation.x, transform.translation.y)
                else {
                    continue;
                };
                let x = (x + d.x).clamp(-150., 150.);
                let y = (y + d.y).clamp(-150., 150.);

                transform.translation = Val2::px(x, y);
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    // Root node filling the whole screen
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(Color::BLACK),
        children![(
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                column_gap: Val::Px(25.0),
                row_gap: Val::Px(25.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            children![
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: Val::Px(10.0),
                        column_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                    GlobalZIndex(1),
                    children![
                        (
                            Button,
                            Node {
                                height: Val::Px(50.0),
                                width: Val::Px(50.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            RotateButton(Rot2::radians(-FRAC_PI_8)),
                            children![(Text::new("<--"), TextColor(Color::BLACK),)]
                        ),
                        (
                            Button,
                            Node {
                                height: Val::Px(50.0),
                                width: Val::Px(50.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            ScaleButton(-0.25),
                            children![(Text::new("-"), TextColor(Color::BLACK),)]
                        ),
                    ]
                ),
                // Target node with its own set of buttons
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        width: Val::Px(300.0),
                        height: Val::Px(300.0),
                        ..default()
                    },
                    BackgroundColor(DARK_GRAY.into()),
                    TargetNode,
                    children![
                        (
                            Button,
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            children![(Text::new("Top"), TextColor(Color::BLACK))]
                        ),
                        (
                            Node {
                                align_self: AlignSelf::Stretch,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            children![
                                (
                                    Button,
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(80.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    BackgroundColor(Color::WHITE),
                                    UiTransform::from_rotation(Rot2::radians(
                                        -std::f32::consts::FRAC_PI_2
                                    )),
                                    children![(Text::new("Left"), TextColor(Color::BLACK),)]
                                ),
                                (
                                    Node {
                                        width: Val::Px(100.),
                                        height: Val::Px(100.),
                                        ..Default::default()
                                    },
                                    ImageNode {
                                        image: asset_server.load("branding/icon.png"),
                                        image_mode: NodeImageMode::Stretch,
                                        ..default()
                                    }
                                ),
                                (
                                    Button,
                                    Node {
                                        width: Val::Px(80.0),
                                        height: Val::Px(80.0),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    UiTransform::from_rotation(Rot2::radians(
                                        core::f32::consts::FRAC_PI_2
                                    )),
                                    BackgroundColor(Color::WHITE),
                                    children![(Text::new("Right"), TextColor(Color::BLACK))]
                                ),
                            ]
                        ),
                        (
                            Button,
                            Node {
                                width: Val::Px(80.0),
                                height: Val::Px(80.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            UiTransform::from_rotation(Rot2::radians(std::f32::consts::PI)),
                            children![(Text::new("Bottom"), TextColor(Color::BLACK),)]
                        ),
                    ]
                ),
                // Right column of controls
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        row_gap: Val::Px(10.0),
                        column_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(10.0)),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                    GlobalZIndex(1),
                    children![
                        (
                            Button,
                            Node {
                                height: Val::Px(50.0),
                                width: Val::Px(50.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            RotateButton(Rot2::radians(FRAC_PI_8)),
                            children![(Text::new("-->"), TextColor(Color::BLACK),)]
                        ),
                        (
                            Button,
                            Node {
                                height: Val::Px(50.0),
                                width: Val::Px(50.0),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(Color::WHITE),
                            ScaleButton(0.25),
                            children![(Text::new("+"), TextColor(Color::BLACK),)]
                        ),
                    ]
                )
            ]
        )],
    ));
}
