//! Demonstration of controlling the run of an application.

use bevy::{
    input::{keyboard::KeyboardInput, ButtonState},
    prelude::*,
    window::{RequestRedraw, WindowFocused},
};

fn main() {
    App::new()
        // By manually control redraw, power-saving applications can be developed
        .insert_resource(WinitSettings {
            redraw_when_tick: true,
            redraw_when_window_event: false,
            redraw_when_device_event: false,
            ..default()
        })
        .add_systems(Update, power_saving)
        .insert_resource(Counter { tick: 0, frame: 0 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(PostStartup, |mut handler: ResMut<WinitHandler>| {
            handler.run();
        })
        .add_systems(UpdateFlow, |mut count: ResMut<Counter>| count.tick += 1)
        .add_systems(RenderFlow, |mut count: ResMut<Counter>| count.frame += 1)
        .add_systems(Update, rotate_cube)
        .add_systems(Control, run_control)
        .add_systems(FrameReady, update_text)
        .run();
}

/// Mimics the behavior of the previous reactive mode when unfocusing.
fn power_saving(mut settings: ResMut<WinitSettings>, mut event: EventReader<WindowFocused>) {
    if let Some(e) = event.into_iter().last() {
        settings.frame_rate_limit = if e.focused { f64::INFINITY } else { 10. };
    }
}

fn run_control(mut handler: ResMut<WinitHandler>, mut input: ResMut<Events<KeyboardInput>>) {
    for e in input.drain() {
        if e.state == ButtonState::Pressed {
            if let Some(key_code) = e.key_code {
                match key_code {
                    KeyCode::Space => {
                        if handler.is_running() {
                            handler.pause();
                            handler.redraw();
                        } else {
                            handler.run();
                        }
                    }
                    KeyCode::Return => {
                        handler.step();
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Component)]
struct Rotator;

/// Rotate the cube to make it clear when the app is updating
fn rotate_cube(time: Res<Time>, mut cube_transform: Query<&mut Transform, With<Rotator>>) {
    for mut transform in &mut cube_transform {
        transform.rotate_x(time.delta_seconds());
        transform.rotate_local_y(time.delta_seconds());
    }
}

#[derive(Resource)]
struct Counter {
    pub tick: u64,
    pub frame: u64,
}

#[derive(Component)]
struct ModeText;

fn update_text(
    count: Res<Counter>,
    handler: Res<WinitHandler>,
    mut query: Query<&mut Text, With<ModeText>>,
) {
    let color = if handler.is_running() {
        Color::GREEN
    } else {
        Color::ORANGE
    };

    let mut text = query.single_mut();
    text.sections[2].value = count.tick.to_string();
    text.sections[4].value = count.frame.to_string();
    for s in &mut text.sections[1..] {
        s.style.color = color;
    }
}

/// Set up a scene with a cube and some text
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event: EventWriter<RequestRedraw>,
) {
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            ..default()
        },
        Rotator,
    ));
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    event.send(RequestRedraw);

    let font_size = 50.0;
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(
                "Press spacebar to pause/resume\nPress enter to tick one step forward \n",
                TextStyle {
                    font_size,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            TextSection::new(
                "Tick: ",
                TextStyle {
                    font_size,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size,
                ..default()
            }),
            TextSection::new(
                ", Frame: ",
                TextStyle {
                    font_size,
                    ..default()
                },
            ),
            TextSection::from_style(TextStyle {
                font_size,
                ..default()
            }),
        ])
        .with_style(Style {
            align_self: AlignSelf::FlexStart,
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(5.0),
            ..default()
        }),
        ModeText,
    ));
}
