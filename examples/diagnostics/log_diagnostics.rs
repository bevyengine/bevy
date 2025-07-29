//! Shows different built-in plugins that logs diagnostics, like frames per second (FPS), to the console.

use bevy::{
    color::palettes,
    diagnostic::{
        DiagnosticPath, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin, LogDiagnosticsState, SystemInformationDiagnosticsPlugin,
    },
    prelude::*,
};

const FRAME_TIME_DIAGNOSTICS: [DiagnosticPath; 3] = [
    FrameTimeDiagnosticsPlugin::FPS,
    FrameTimeDiagnosticsPlugin::FRAME_COUNT,
    FrameTimeDiagnosticsPlugin::FRAME_TIME,
];
const ENTITY_COUNT_DIAGNOSTICS: [DiagnosticPath; 1] = [EntityCountDiagnosticsPlugin::ENTITY_COUNT];
const SYSTEM_INFO_DIAGNOSTICS: [DiagnosticPath; 4] = [
    SystemInformationDiagnosticsPlugin::PROCESS_CPU_USAGE,
    SystemInformationDiagnosticsPlugin::PROCESS_MEM_USAGE,
    SystemInformationDiagnosticsPlugin::SYSTEM_CPU_USAGE,
    SystemInformationDiagnosticsPlugin::SYSTEM_MEM_USAGE,
];

fn main() {
    App::new()
        .add_plugins((
            // The diagnostics plugins need to be added after DefaultPlugins as they use e.g. the time plugin for timestamps.
            DefaultPlugins,
            // Adds a system that prints diagnostics to the console.
            // The other diagnostics plugins can still be used without this if you want to use them in an ingame overlay for example.
            LogDiagnosticsPlugin::default(),
            // Adds frame time, FPS and frame count diagnostics.
            FrameTimeDiagnosticsPlugin::default(),
            // Adds an entity count diagnostic.
            EntityCountDiagnosticsPlugin::default(),
            // Adds cpu and memory usage diagnostics for systems and the entire game process.
            SystemInformationDiagnosticsPlugin,
            // Forwards various diagnostics from the render app to the main app.
            // These are pretty verbose but can be useful to pinpoint performance issues.
            bevy::render::diagnostic::RenderDiagnosticsPlugin,
        ))
        // No rendering diagnostics are emitted unless something is drawn to the screen,
        // so we spawn a small scene.
        .add_systems(Startup, setup)
        .add_systems(Update, filters_inputs)
        .add_systems(
            Update,
            update_commands.run_if(
                resource_exists_and_changed::<LogDiagnosticsStatus>
                    .or(resource_exists_and_changed::<LogDiagnosticsFilters>),
            ),
        )
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.init_resource::<LogDiagnosticsFilters>();
    commands.init_resource::<LogDiagnosticsStatus>();

    commands.spawn((
        LogDiagnosticsCommands,
        Node {
            top: Val::Px(5.),
            left: Val::Px(5.),
            flex_direction: FlexDirection::Column,
            ..default()
        },
    ));
}

fn filters_inputs(
    keys: Res<ButtonInput<KeyCode>>,
    mut status: ResMut<LogDiagnosticsStatus>,
    mut filters: ResMut<LogDiagnosticsFilters>,
    mut log_state: ResMut<LogDiagnosticsState>,
) {
    if keys.just_pressed(KeyCode::KeyQ) {
        *status = match *status {
            LogDiagnosticsStatus::Enabled => {
                log_state.disable_filtering();
                LogDiagnosticsStatus::Disabled
            }
            LogDiagnosticsStatus::Disabled => {
                log_state.enable_filtering();
                if filters.frame_time {
                    enable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
                }
                if filters.entity_count {
                    enable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
                }
                if filters.system_info {
                    enable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
                }
                LogDiagnosticsStatus::Enabled
            }
        };
    }

    let enabled = *status == LogDiagnosticsStatus::Enabled;
    if keys.just_pressed(KeyCode::Digit1) {
        filters.frame_time = !filters.frame_time;
        if enabled {
            if filters.frame_time {
                enable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, FRAME_TIME_DIAGNOSTICS);
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit2) {
        filters.entity_count = !filters.entity_count;
        if enabled {
            if filters.entity_count {
                enable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, ENTITY_COUNT_DIAGNOSTICS);
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit3) {
        filters.system_info = !filters.system_info;
        if enabled {
            if filters.system_info {
                enable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
            } else {
                disable_filters(&mut log_state, SYSTEM_INFO_DIAGNOSTICS);
            }
        }
    }
}

fn enable_filters(
    log_state: &mut LogDiagnosticsState,
    diagnostics: impl IntoIterator<Item = DiagnosticPath>,
) {
    log_state.extend_filter(diagnostics);
}

fn disable_filters(
    log_state: &mut LogDiagnosticsState,
    diagnostics: impl IntoIterator<Item = DiagnosticPath>,
) {
    for diagnostic in diagnostics {
        log_state.remove_filter(&diagnostic);
    }
}

fn update_commands(
    mut commands: Commands,
    log_commands: Single<Entity, With<LogDiagnosticsCommands>>,
    status: Res<LogDiagnosticsStatus>,
    filters: Res<LogDiagnosticsFilters>,
) {
    let enabled = *status == LogDiagnosticsStatus::Enabled;
    let alpha = if enabled { 1. } else { 0.25 };
    let enabled_color = |enabled| {
        if enabled {
            Color::from(palettes::tailwind::GREEN_400)
        } else {
            Color::from(palettes::tailwind::RED_400)
        }
    };
    commands
        .entity(*log_commands)
        .despawn_related::<Children>()
        .insert(children![
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.),
                    ..default()
                },
                children![
                    Text::new("[Q] Toggle filtering:"),
                    (
                        Text::new(format!("{:?}", *status)),
                        TextColor(enabled_color(enabled))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.),
                    ..default()
                },
                children![
                    (
                        Text::new("[1] Frame times:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.frame_time)),
                        TextColor(enabled_color(filters.frame_time).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.),
                    ..default()
                },
                children![
                    (
                        Text::new("[2] Entity count:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.entity_count)),
                        TextColor(enabled_color(filters.entity_count).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.),
                    ..default()
                },
                children![
                    (
                        Text::new("[3] System info:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new(format!("{:?}", filters.system_info)),
                        TextColor(enabled_color(filters.system_info).with_alpha(alpha))
                    )
                ]
            ),
            (
                Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(5.),
                    ..default()
                },
                children![
                    (
                        Text::new("[4] Render diagnostics:"),
                        TextColor(Color::WHITE.with_alpha(alpha))
                    ),
                    (
                        Text::new("Private"),
                        TextColor(enabled_color(false).with_alpha(alpha))
                    )
                ]
            ),
        ]);
}

#[derive(Debug, Default, PartialEq, Eq, Resource)]
enum LogDiagnosticsStatus {
    /// No filtering, showing all logs
    #[default]
    Disabled,
    /// Filtering enabled, showing only subset of logs
    Enabled,
}

#[derive(Default, Resource)]
struct LogDiagnosticsFilters {
    frame_time: bool,
    entity_count: bool,
    system_info: bool,
    #[expect(
        dead_code,
        reason = "Currently the diagnostic paths referent to RenderDiagnosticPlugin are private"
    )]
    render_diagnostics: bool,
}

#[derive(Component)]
/// Marks the UI node that has instructions on how to change the filtering
struct LogDiagnosticsCommands;
