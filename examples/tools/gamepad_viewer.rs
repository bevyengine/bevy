//! Shows a visualization of gamepad buttons, sticks, and triggers

use std::f32::consts::PI;

use bevy::{
    input::gamepad::{GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnectionEvent},
    prelude::*,
    sprite::Anchor,
};

const BUTTON_RADIUS: f32 = 25.;
const BUTTON_CLUSTER_RADIUS: f32 = 50.;
const START_SIZE: Vec2 = Vec2::new(30., 15.);
const TRIGGER_SIZE: Vec2 = Vec2::new(70., 20.);
const STICK_BOUNDS_SIZE: f32 = 100.;

const BUTTONS_X: f32 = 150.;
const BUTTONS_Y: f32 = 80.;
const STICKS_X: f32 = 150.;
const STICKS_Y: f32 = -135.;

const NORMAL_BUTTON_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);
const ACTIVE_BUTTON_COLOR: Color = Color::srgb(0.5, 0., 0.5);
const LIVE_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
const DEAD_COLOR: Color = Color::srgb(0.13, 0.13, 0.13);

#[derive(Component, Deref)]
struct ReactTo(GamepadButton);
#[derive(Component)]
struct MoveWithAxes {
    x_axis: GamepadAxis,
    y_axis: GamepadAxis,
    scale: f32,
}
#[derive(Component)]
struct TextWithAxes {
    x_axis: GamepadAxis,
    y_axis: GamepadAxis,
}
#[derive(Component, Deref)]
struct TextWithButtonValue(GamepadButton);

#[derive(Component)]
struct ConnectedGamepadsText;

#[derive(Resource)]
struct ButtonMaterials {
    normal: MeshMaterial2d<ColorMaterial>,
    active: MeshMaterial2d<ColorMaterial>,
}
impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        Self {
            normal: world.add_asset(NORMAL_BUTTON_COLOR).into(),
            active: world.add_asset(ACTIVE_BUTTON_COLOR).into(),
        }
    }
}
#[derive(Resource)]
struct ButtonMeshes {
    circle: Mesh2d,
    triangle: Mesh2d,
    start_pause: Mesh2d,
    trigger: Mesh2d,
}
impl FromWorld for ButtonMeshes {
    fn from_world(world: &mut World) -> Self {
        Self {
            circle: world.add_asset(Circle::new(BUTTON_RADIUS)).into(),
            triangle: world
                .add_asset(RegularPolygon::new(BUTTON_RADIUS, 3))
                .into(),
            start_pause: world.add_asset(Rectangle::from_size(START_SIZE)).into(),
            trigger: world.add_asset(Rectangle::from_size(TRIGGER_SIZE)).into(),
        }
    }
}

#[derive(Bundle)]
struct GamepadButtonBundle {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
    react_to: ReactTo,
}

impl GamepadButtonBundle {
    pub fn new(
        button_type: GamepadButton,
        mesh: Mesh2d,
        material: MeshMaterial2d<ColorMaterial>,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            mesh,
            material,
            transform: Transform::from_xyz(x, y, 0.),
            react_to: ReactTo(button_type),
        }
    }

    pub fn with_rotation(mut self, angle: f32) -> Self {
        self.transform.rotation = Quat::from_rotation_z(angle);
        self
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ButtonMaterials>()
        .init_resource::<ButtonMeshes>()
        .add_systems(
            Startup,
            (setup, setup_sticks, setup_triggers, setup_connected),
        )
        .add_systems(
            Update,
            (
                update_buttons,
                update_button_values,
                update_axes,
                update_connected,
            ),
        )
        .run();
}

fn setup(mut commands: Commands, meshes: Res<ButtonMeshes>, materials: Res<ButtonMaterials>) {
    commands.spawn(Camera2dBundle::default());

    // Buttons

    commands
        .spawn(SpatialBundle {
            transform: Transform::from_xyz(BUTTONS_X, BUTTONS_Y, 0.),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(GamepadButtonBundle::new(
                GamepadButton::North,
                meshes.circle.clone(),
                materials.normal.clone(),
                0.,
                BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButton::South,
                meshes.circle.clone(),
                materials.normal.clone(),
                0.,
                -BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButton::West,
                meshes.circle.clone(),
                materials.normal.clone(),
                -BUTTON_CLUSTER_RADIUS,
                0.,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButton::East,
                meshes.circle.clone(),
                materials.normal.clone(),
                BUTTON_CLUSTER_RADIUS,
                0.,
            ));
        });

    // Start and Pause

    commands.spawn(GamepadButtonBundle::new(
        GamepadButton::Select,
        meshes.start_pause.clone(),
        materials.normal.clone(),
        -30.,
        BUTTONS_Y,
    ));

    commands.spawn(GamepadButtonBundle::new(
        GamepadButton::Start,
        meshes.start_pause.clone(),
        materials.normal.clone(),
        30.,
        BUTTONS_Y,
    ));

    // D-Pad

    commands
        .spawn(SpatialBundle {
            transform: Transform::from_xyz(-BUTTONS_X, BUTTONS_Y, 0.),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(GamepadButtonBundle::new(
                GamepadButton::DPadUp,
                meshes.triangle.clone(),
                materials.normal.clone(),
                0.,
                BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButton::DPadDown,
                    meshes.triangle.clone(),
                    materials.normal.clone(),
                    0.,
                    -BUTTON_CLUSTER_RADIUS,
                )
                .with_rotation(PI),
            );
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButton::DPadLeft,
                    meshes.triangle.clone(),
                    materials.normal.clone(),
                    -BUTTON_CLUSTER_RADIUS,
                    0.,
                )
                .with_rotation(PI / 2.),
            );
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButton::DPadRight,
                    meshes.triangle.clone(),
                    materials.normal.clone(),
                    BUTTON_CLUSTER_RADIUS,
                    0.,
                )
                .with_rotation(-PI / 2.),
            );
        });

    // Triggers

    commands.spawn(GamepadButtonBundle::new(
        GamepadButton::LeftTrigger,
        meshes.trigger.clone(),
        materials.normal.clone(),
        -BUTTONS_X,
        BUTTONS_Y + 115.,
    ));

    commands.spawn(GamepadButtonBundle::new(
        GamepadButton::RightTrigger,
        meshes.trigger.clone(),
        materials.normal.clone(),
        BUTTONS_X,
        BUTTONS_Y + 115.,
    ));
}

fn setup_sticks(
    mut commands: Commands,
    meshes: Res<ButtonMeshes>,
    materials: Res<ButtonMaterials>,
) {
    // NOTE: This stops making sense because in entities because there isn't a "global" default,
    // instead each gamepad has its own default setting
    let gamepad_settings = GamepadSettings::default();
    let dead_upper =
        STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.deadzone_upperbound();
    let dead_lower =
        STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.deadzone_lowerbound();
    let dead_size = dead_lower.abs() + dead_upper.abs();
    let dead_mid = (dead_lower + dead_upper) / 2.0;

    let live_upper =
        STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.livezone_upperbound();
    let live_lower =
        STICK_BOUNDS_SIZE * gamepad_settings.default_axis_settings.livezone_lowerbound();
    let live_size = live_lower.abs() + live_upper.abs();
    let live_mid = (live_lower + live_upper) / 2.0;

    let mut spawn_stick = |x_pos, y_pos, x_axis, y_axis, button| {
        commands
            .spawn(SpatialBundle {
                transform: Transform::from_xyz(x_pos, y_pos, 0.),
                ..default()
            })
            .with_children(|parent| {
                // full extent
                parent.spawn(SpriteBundle {
                    sprite: Sprite {
                        custom_size: Some(Vec2::splat(STICK_BOUNDS_SIZE * 2.)),
                        color: DEAD_COLOR,
                        ..default()
                    },
                    ..default()
                });
                // live zone
                parent.spawn(SpriteBundle {
                    transform: Transform::from_xyz(live_mid, live_mid, 2.),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(live_size, live_size)),
                        color: LIVE_COLOR,
                        ..default()
                    },
                    ..default()
                });
                // dead zone
                parent.spawn(SpriteBundle {
                    transform: Transform::from_xyz(dead_mid, dead_mid, 3.),
                    sprite: Sprite {
                        custom_size: Some(Vec2::new(dead_size, dead_size)),
                        color: DEAD_COLOR,
                        ..default()
                    },
                    ..default()
                });
                // text
                let style = TextStyle {
                    font_size: 13.,
                    ..default()
                };
                parent.spawn((
                    Text2dBundle {
                        transform: Transform::from_xyz(0., STICK_BOUNDS_SIZE + 2., 4.),
                        text: Text::from_sections([
                            TextSection {
                                value: format!("{:.3}", 0.),
                                style: style.clone(),
                            },
                            TextSection {
                                value: ", ".to_string(),
                                style: style.clone(),
                            },
                            TextSection {
                                value: format!("{:.3}", 0.),
                                style,
                            },
                        ]),
                        text_anchor: Anchor::BottomCenter,
                        ..default()
                    },
                    TextWithAxes { x_axis, y_axis },
                ));
                // cursor
                parent.spawn((
                    meshes.circle.clone(),
                    materials.normal.clone(),
                    Transform::from_xyz(0., 0., 5.).with_scale(Vec2::splat(0.15).extend(1.)),
                    MoveWithAxes {
                        x_axis,
                        y_axis,
                        scale: STICK_BOUNDS_SIZE,
                    },
                    ReactTo(button),
                ));
            });
    };

    spawn_stick(
        -STICKS_X,
        STICKS_Y,
        GamepadAxis::LeftStickX,
        GamepadAxis::LeftStickY,
        GamepadButton::LeftThumb,
    );
    spawn_stick(
        STICKS_X,
        STICKS_Y,
        GamepadAxis::RightStickX,
        GamepadAxis::RightStickY,
        GamepadButton::RightThumb,
    );
}

fn setup_triggers(
    mut commands: Commands,
    meshes: Res<ButtonMeshes>,
    materials: Res<ButtonMaterials>,
) {
    let mut spawn_trigger = |x, y, button_type| {
        commands
            .spawn(GamepadButtonBundle::new(
                button_type,
                meshes.trigger.clone(),
                materials.normal.clone(),
                x,
                y,
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text2dBundle {
                        transform: Transform::from_xyz(0., 0., 1.),
                        text: Text::from_section(
                            format!("{:.3}", 0.),
                            TextStyle {
                                font_size: 13.,
                                ..default()
                            },
                        ),
                        ..default()
                    },
                    TextWithButtonValue(button_type),
                ));
            });
    };

    spawn_trigger(-BUTTONS_X, BUTTONS_Y + 145., GamepadButton::LeftTrigger2);
    spawn_trigger(BUTTONS_X, BUTTONS_Y + 145., GamepadButton::RightTrigger2);
}

fn setup_connected(mut commands: Commands) {
    let text_style = TextStyle::default();

    commands.spawn((
        TextBundle {
            text: Text::from_sections([
                TextSection {
                    value: "Connected Gamepads:\n".to_string(),
                    style: text_style.clone(),
                },
                TextSection {
                    value: "None".to_string(),
                    style: text_style,
                },
            ]),
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(12.),
                left: Val::Px(12.),
                ..default()
            },
            ..default()
        },
        ConnectedGamepadsText,
    ));
}

fn update_buttons(
    gamepads: Query<&Gamepad>,
    materials: Res<ButtonMaterials>,
    mut query: Query<(&mut MeshMaterial2d<ColorMaterial>, &ReactTo)>,
) {
    for buttons in &gamepads {
        for (mut handle, react_to) in query.iter_mut() {
            if buttons.just_pressed(**react_to) {
                *handle = materials.active.clone();
            }
            if buttons.just_released(**react_to) {
                *handle = materials.normal.clone();
            }
        }
    }
}
fn update_button_values(
    mut events: EventReader<GamepadButtonChangedEvent>,
    mut query: Query<(&mut Text, &TextWithButtonValue)>,
) {
    for button_event in events.read() {
        for (mut text, text_with_button_value) in query.iter_mut() {
            if button_event.button == **text_with_button_value {
                text.sections[0].value = format!("{:.3}", button_event.value);
            }
        }
    }
}

fn update_axes(
    mut axis_events: EventReader<GamepadAxisChangedEvent>,
    mut query: Query<(&mut Transform, &MoveWithAxes)>,
    mut text_query: Query<(&mut Text, &TextWithAxes)>,
) {
    for axis_event in axis_events.read() {
        let axis_type = axis_event.axis;
        let value = axis_event.value;
        for (mut transform, move_with) in query.iter_mut() {
            if axis_type == move_with.x_axis {
                transform.translation.x = value * move_with.scale;
            }
            if axis_type == move_with.y_axis {
                transform.translation.y = value * move_with.scale;
            }
        }
        for (mut text, text_with_axes) in text_query.iter_mut() {
            if axis_type == text_with_axes.x_axis {
                text.sections[0].value = format!("{value:.3}");
            }
            if axis_type == text_with_axes.y_axis {
                text.sections[2].value = format!("{value:.3}");
            }
        }
    }
}

fn update_connected(
    mut connected: EventReader<GamepadConnectionEvent>,
    gamepads: Query<(Entity, &Gamepad)>,
    mut query: Query<&mut Text, With<ConnectedGamepadsText>>,
) {
    if connected.is_empty() {
        return;
    }
    connected.clear();
    let mut text = query.single_mut();

    let formatted = gamepads
        .iter()
        .map(|(entity, gamepad)| format!("{} - {}", entity, gamepad.name()))
        .collect::<Vec<_>>()
        .join("\n");

    text.sections[1].value = if !formatted.is_empty() {
        formatted
    } else {
        "None".to_string()
    }
}
