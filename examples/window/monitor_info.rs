//! Displays information about available monitors (displays).

use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{ExitCondition, Monitor, WindowMode, WindowRef},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        }))
        .add_systems(Update, (update, close_on_esc))
        .run();
}

#[derive(Component)]
struct MonitorRef(Entity);

fn update(
    mut commands: Commands,
    monitors_added: Query<(Entity, &Monitor), Added<Monitor>>,
    mut monitors_removed: RemovedComponents<Monitor>,
    monitor_refs: Query<(Entity, &MonitorRef)>,
) {
    for (entity, monitor) in monitors_added.iter() {
        // Spawn a new window on each monitor
        let name = monitor.name.clone().unwrap_or_else(|| "<no name>".into());
        let size = format!("{}x{}px", monitor.physical_height, monitor.physical_width);
        let hz = monitor
            .refresh_rate_millihertz
            .map(|x| format!("{}Hz", x as f32 / 1000.0))
            .unwrap_or_else(|| "<unknown>".into());
        let position = format!(
            "x={} y={}",
            monitor.physical_position.x, monitor.physical_position.y
        );
        let scale = format!("{:.2}", monitor.scale_factor);

        let window = commands
            .spawn((
                Window {
                    title: name.clone(),
                    mode: WindowMode::Fullscreen(
                        MonitorSelection::Entity(entity),
                        VideoModeSelection::Current,
                    ),
                    position: WindowPosition::Centered(MonitorSelection::Entity(entity)),
                    ..default()
                },
                MonitorRef(entity),
            ))
            .id();

        let camera = commands
            .spawn((
                Camera2d,
                Camera {
                    target: RenderTarget::Window(WindowRef::Entity(window)),
                    ..default()
                },
            ))
            .id();

        let info_text = format!(
            "Monitor: {name}\nSize: {size}\nRefresh rate: {hz}\nPosition: {position}\nScale: {scale}\n\n",
        );
        commands.spawn((
            Text(info_text),
            Node {
                position_type: PositionType::Relative,
                height: percent(100),
                width: percent(100),
                ..default()
            },
            UiTargetCamera(camera),
            MonitorRef(entity),
        ));
    }

    // Remove windows for removed monitors
    for monitor_entity in monitors_removed.read() {
        for (ref_entity, monitor_ref) in monitor_refs.iter() {
            if monitor_ref.0 == monitor_entity {
                commands.entity(ref_entity).despawn();
            }
        }
    }
}

fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}
