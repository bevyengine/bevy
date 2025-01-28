//! Using different [`MainUpdateMode`] and [`RenderUpdateMode`] to run the application at various
//! different TPS and FPS.

use std::{
    sync::LazyLock,
    time::{Duration, Instant},
};

use std::f32::consts::TAU;

use bevy::{
    prelude::*,
    render::{Render, RenderApp},
    winit::{MainUpdateMode, RenderUpdateMode, WinitSettings},
};

fn main() {
    let mut app = App::new();

    let (render_time_tx, render_time_rx) = crossbeam_channel::unbounded();

    app.insert_resource(RenderTimeRx(render_time_rx))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (switch_update_modes, rotate_cube));

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        panic!("Failed to get render app");
    };

    render_app
        .insert_resource(RenderTimeTx(render_time_tx))
        .add_systems(Render, update_render_duration);

    app.run();
}

// Define a component to designate a rotation speed to an entity.
#[derive(Component)]
struct Rotatable {
    speed: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a cube to rotate.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_translation(Vec3::ZERO),
        Rotatable { speed: 0.3 },
    ));

    // Spawn a camera looking at the entities to show what's happening in this example.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Add a light source so we can see clearly.
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn(Text::default());
}

#[derive(Resource)]
struct RenderTimeTx(crossbeam_channel::Sender<Duration>);

#[derive(Resource)]
struct RenderTimeRx(crossbeam_channel::Receiver<Duration>);

fn switch_update_modes(
    mut offset: Local<usize>,
    mut last_instant: Local<Option<Instant>>,
    mut last_render_duration: Local<Option<Duration>>,
    mut settings: ResMut<WinitSettings>,
    render_time_rx: Res<RenderTimeRx>,
    keyboard: Res<ButtonInput<KeyCode>>,
    text: Query<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    static SETTINGS: LazyLock<[(&str, WinitSettings); 4]> = LazyLock::new(|| {
        [
            ("Game (OnEachFrame, Continuous)", WinitSettings::game()),
            (
                "Desktop App (Reactive, OnEachMainUpdate)",
                WinitSettings::desktop_app(),
            ),
            (
                "Desktop App 2 (Reactive, OnEachMainUpdate { capped at 6 fps })",
                WinitSettings {
                    focused_mode: (
                        MainUpdateMode::reactive(Duration::from_secs(5)),
                        RenderUpdateMode::OnEachMainUpdate {
                            min_frametime: Some(Duration::from_secs_f64(1.0 / 6.0)),
                        },
                    ),
                    unfocused_mode: (
                        MainUpdateMode::reactive(Duration::from_secs(5)),
                        RenderUpdateMode::OnEachMainUpdate {
                            min_frametime: Some(Duration::from_secs_f64(1.0 / 6.0)),
                        },
                    ),
                },
            ),
            (
                "Simulation (Continuous, Fixed(6 FPS))",
                WinitSettings {
                    focused_mode: (
                        MainUpdateMode::Continuous,
                        RenderUpdateMode::Fixed(Duration::from_secs_f64(1.0 / 6.0)),
                    ),
                    unfocused_mode: (
                        MainUpdateMode::Continuous,
                        RenderUpdateMode::Fixed(Duration::from_secs_f64(1.0 / 6.0)),
                    ),
                },
            ),
        ]
    });

    let mode_name = SETTINGS[*offset % SETTINGS.len()].0;

    if let Ok(duration) = render_time_rx.0.try_recv() {
        *last_render_duration = Some(duration);
    }

    if let Some((last_instant, render_duration)) =
        last_instant.as_ref().zip(last_render_duration.as_ref())
    {
        *writer.text(text.single(), 0) = format!(
            "Mode: {mode_name}\nTime between main update: {:?}, Time between render update: {:?}",
            last_instant.elapsed(),
            render_duration
        );
    }

    *last_instant = Some(Instant::now());

    if keyboard.just_pressed(KeyCode::Space) {
        *offset += 1;
    }

    *settings = SETTINGS[*offset % SETTINGS.len()].1;
}

fn update_render_duration(
    mut last_instant: Local<Option<Instant>>,
    render_time_tx: Res<RenderTimeTx>,
) {
    if let Some(last_instant) = last_instant.as_mut() {
        render_time_tx
            .0
            .send(last_instant.elapsed())
            .expect("Failed to send time");
    }

    *last_instant = Some(Instant::now());
}

// This system will rotate any entity in the scene with a Rotatable component around its y-axis.
fn rotate_cube(mut cubes: Query<(&mut Transform, &Rotatable)>, timer: Res<Time>) {
    for (mut transform, cube) in &mut cubes {
        // The speed is first multiplied by TAU which is a full rotation (360deg) in radians,
        // and then multiplied by delta_secs which is the time that passed last frame.
        // In other words. Speed is equal to the amount of rotations per second.
        transform.rotate_y(cube.speed * TAU * timer.delta_secs());
    }
}
