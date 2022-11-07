//! Shows a visualization of gamepad buttons, sticks, and triggers

use std::f32::consts::PI;

use bevy::{
    input::gamepad::{GamepadButton, GamepadSettings},
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
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

const NORMAL_BUTTON_COLOR: Color = Color::rgb(0.2, 0.2, 0.2);
const ACTIVE_BUTTON_COLOR: Color = Color::PURPLE;
const LIVE_COLOR: Color = Color::rgb(0.4, 0.4, 0.4);
const DEAD_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const EXTENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const TEXT_COLOR: Color = Color::WHITE;

#[derive(Component, Deref)]
struct ReactTo(GamepadButtonType);
#[derive(Component)]
struct MoveWithAxes {
    x_axis: GamepadAxisType,
    y_axis: GamepadAxisType,
    scale: f32,
}
#[derive(Component)]
struct TextWithAxes {
    x_axis: GamepadAxisType,
    y_axis: GamepadAxisType,
}
#[derive(Component, Deref)]
struct TextWithButtonValue(GamepadButtonType);

#[derive(Component)]
struct ConnectedGamepadsText;

#[derive(Resource)]
struct ButtonMaterials {
    normal: Handle<ColorMaterial>,
    active: Handle<ColorMaterial>,
}
impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.resource_mut::<Assets<ColorMaterial>>();
        Self {
            normal: materials.add(ColorMaterial::from(NORMAL_BUTTON_COLOR)),
            active: materials.add(ColorMaterial::from(ACTIVE_BUTTON_COLOR)),
        }
    }
}
#[derive(Resource)]
struct ButtonMeshes {
    circle: Mesh2dHandle,
    triangle: Mesh2dHandle,
    start_pause: Mesh2dHandle,
    trigger: Mesh2dHandle,
}
impl FromWorld for ButtonMeshes {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        Self {
            circle: meshes.add(shape::Circle::new(BUTTON_RADIUS).into()).into(),
            triangle: meshes
                .add(shape::RegularPolygon::new(BUTTON_RADIUS, 3).into())
                .into(),
            start_pause: meshes.add(shape::Quad::new(START_SIZE).into()).into(),
            trigger: meshes.add(shape::Quad::new(TRIGGER_SIZE).into()).into(),
        }
    }
}
#[derive(Resource, Deref)]
struct FontHandle(Handle<Font>);
impl FromWorld for FontHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(asset_server.load("fonts/FiraSans-Bold.ttf"))
    }
}

#[derive(Bundle)]
struct GamepadButtonBundle {
    mesh_bundle: MaterialMesh2dBundle<ColorMaterial>,
    react_to: ReactTo,
}

impl GamepadButtonBundle {
    pub fn new(
        button_type: GamepadButtonType,
        mesh: Mesh2dHandle,
        material: Handle<ColorMaterial>,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            mesh_bundle: MaterialMesh2dBundle {
                mesh,
                material,
                transform: Transform::from_xyz(x, y, 0.),
                ..default()
            },
            react_to: ReactTo(button_type),
        }
    }

    pub fn with_rotation(mut self, angle: f32) -> Self {
        self.mesh_bundle.transform.rotation = Quat::from_rotation_z(angle);
        self
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<ButtonMaterials>()
        .init_resource::<ButtonMeshes>()
        .init_resource::<FontHandle>()
        .add_startup_system(setup)
        .add_startup_system(setup_sticks)
        .add_startup_system(setup_triggers)
        .add_startup_system(setup_connected)
        .add_system(update_buttons)
        .add_system(update_button_values)
        .add_system(update_axes)
        .add_system(update_connected)
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
                GamepadButtonType::North,
                meshes.circle.clone(),
                materials.normal.clone(),
                0.,
                BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButtonType::South,
                meshes.circle.clone(),
                materials.normal.clone(),
                0.,
                -BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButtonType::West,
                meshes.circle.clone(),
                materials.normal.clone(),
                -BUTTON_CLUSTER_RADIUS,
                0.,
            ));
            parent.spawn(GamepadButtonBundle::new(
                GamepadButtonType::East,
                meshes.circle.clone(),
                materials.normal.clone(),
                BUTTON_CLUSTER_RADIUS,
                0.,
            ));
        });

    // Start and Pause

    commands.spawn(GamepadButtonBundle::new(
        GamepadButtonType::Select,
        meshes.start_pause.clone(),
        materials.normal.clone(),
        -30.,
        BUTTONS_Y,
    ));

    commands.spawn(GamepadButtonBundle::new(
        GamepadButtonType::Start,
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
                GamepadButtonType::DPadUp,
                meshes.triangle.clone(),
                materials.normal.clone(),
                0.,
                BUTTON_CLUSTER_RADIUS,
            ));
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButtonType::DPadDown,
                    meshes.triangle.clone(),
                    materials.normal.clone(),
                    0.,
                    -BUTTON_CLUSTER_RADIUS,
                )
                .with_rotation(PI),
            );
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButtonType::DPadLeft,
                    meshes.triangle.clone(),
                    materials.normal.clone(),
                    -BUTTON_CLUSTER_RADIUS,
                    0.,
                )
                .with_rotation(PI / 2.),
            );
            parent.spawn(
                GamepadButtonBundle::new(
                    GamepadButtonType::DPadRight,
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
        GamepadButtonType::LeftTrigger,
        meshes.trigger.clone(),
        materials.normal.clone(),
        -BUTTONS_X,
        BUTTONS_Y + 115.,
    ));

    commands.spawn(GamepadButtonBundle::new(
        GamepadButtonType::RightTrigger,
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
    gamepad_settings: Res<GamepadSettings>,
    font: Res<FontHandle>,
) {
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
                        color: EXTENT_COLOR,
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
                    font_size: 16.,
                    color: TEXT_COLOR,
                    font: font.clone(),
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
                        ])
                        .with_alignment(TextAlignment::BOTTOM_CENTER),
                        ..default()
                    },
                    TextWithAxes { x_axis, y_axis },
                ));
                // cursor
                parent.spawn((
                    MaterialMesh2dBundle {
                        mesh: meshes.circle.clone(),
                        material: materials.normal.clone(),
                        transform: Transform::from_xyz(0., 0., 5.)
                            .with_scale(Vec2::splat(0.2).extend(1.)),
                        ..default()
                    },
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
        GamepadAxisType::LeftStickX,
        GamepadAxisType::LeftStickY,
        GamepadButtonType::LeftThumb,
    );
    spawn_stick(
        STICKS_X,
        STICKS_Y,
        GamepadAxisType::RightStickX,
        GamepadAxisType::RightStickY,
        GamepadButtonType::RightThumb,
    );
}

fn setup_triggers(
    mut commands: Commands,
    meshes: Res<ButtonMeshes>,
    materials: Res<ButtonMaterials>,
    font: Res<FontHandle>,
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
                                font: font.clone(),
                                font_size: 16.,
                                color: TEXT_COLOR,
                            },
                        )
                        .with_alignment(TextAlignment::CENTER),
                        ..default()
                    },
                    TextWithButtonValue(button_type),
                ));
            });
    };

    spawn_trigger(
        -BUTTONS_X,
        BUTTONS_Y + 145.,
        GamepadButtonType::LeftTrigger2,
    );
    spawn_trigger(
        BUTTONS_X,
        BUTTONS_Y + 145.,
        GamepadButtonType::RightTrigger2,
    );
}

fn setup_connected(mut commands: Commands, font: Res<FontHandle>) {
    let style = TextStyle {
        color: TEXT_COLOR,
        font_size: 30.,
        font: font.clone(),
    };
    commands.spawn((
        TextBundle::from_sections([
            TextSection {
                value: "Connected Gamepads:\n".to_string(),
                style: style.clone(),
            },
            TextSection {
                value: "None".to_string(),
                style,
            },
        ]),
        ConnectedGamepadsText,
    ));
}

fn update_buttons(
    gamepads: Res<Gamepads>,
    button_inputs: Res<Input<GamepadButton>>,
    materials: Res<ButtonMaterials>,
    mut query: Query<(&mut Handle<ColorMaterial>, &ReactTo)>,
) {
    for gamepad in gamepads.iter() {
        for (mut handle, react_to) in query.iter_mut() {
            if button_inputs.just_pressed(GamepadButton::new(gamepad, **react_to)) {
                *handle = materials.active.clone();
            }
            if button_inputs.just_released(GamepadButton::new(gamepad, **react_to)) {
                *handle = materials.normal.clone();
            }
        }
    }
}

fn update_button_values(
    mut events: EventReader<GamepadEvent>,
    mut query: Query<(&mut Text, &TextWithButtonValue)>,
) {
    for event in events.iter() {
        if let GamepadEventType::ButtonChanged(button_type, value) = event.event_type {
            for (mut text, text_with_button_value) in query.iter_mut() {
                if button_type == **text_with_button_value {
                    text.sections[0].value = format!("{value:.3}");
                }
            }
        }
    }
}

fn update_axes(
    mut events: EventReader<GamepadEvent>,
    mut query: Query<(&mut Transform, &MoveWithAxes)>,
    mut text_query: Query<(&mut Text, &TextWithAxes)>,
) {
    for event in events.iter() {
        if let GamepadEventType::AxisChanged(axis_type, value) = event.event_type {
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
}

fn update_connected(
    gamepads: Res<Gamepads>,
    mut query: Query<&mut Text, With<ConnectedGamepadsText>>,
) {
    if !gamepads.is_changed() {
        return;
    }

    let mut text = query.single_mut();

    let formatted = gamepads
        .iter()
        .map(|g| format!("- {}", gamepads.name(g).unwrap()))
        .collect::<Vec<_>>()
        .join("\n");

    text.sections[1].value = if !formatted.is_empty() {
        formatted
    } else {
        "None".to_string()
    }
}
