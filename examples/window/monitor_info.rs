//! Displays information about available monitors (displays).

use bevy::window::{ExitCondition, WindowRef};
use bevy::{prelude::*, window::Monitor};
use bevy::render::camera::RenderTarget;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: None,
            exit_condition: ExitCondition::DontExit,
            ..default()
        }))
        .add_systems(Update, update)
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
        let mut text = TextBundle::default();
        let mut info = Vec::new();

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
        info.push(TextSection {
            value: format!(
                "Monitor: {name}\nSize: {size}\nRefresh rate: {hz}\nPosition: {position}\nScale: {scale}\n\n",
            ),
            ..default()
        });

        text.text.sections = info;

        let window = commands
            .spawn((
                Window {
                    title: name,
                    position: WindowPosition::Centered(MonitorSelection::Entity(entity)),
                    ..default()
                },
                MonitorRef(entity),
            ))
            .id();

        let camera = commands.spawn(Camera2dBundle {
            camera: Camera {
                target: RenderTarget::Window(WindowRef::Entity(window)),
                ..default()
            },
            ..default()
        }).id();

        commands.spawn((text, TargetCamera(camera), MonitorRef(entity)));
    }

    // Remove windows for removed monitors
    for monitor_entity in monitors_removed.read() {
        for (ref_entity, monitor_ref) in monitor_refs.iter() {
            if monitor_ref.0 == monitor_entity {
                commands.entity(ref_entity).despawn_recursive();
            }
        }
    }
}
