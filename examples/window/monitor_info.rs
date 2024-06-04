//! Displays information about available monitors (displays).

use bevy::{prelude::*, window::Monitor};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, text_update_system)
        .run();
}

#[derive(Component)]
struct MonitorInfoText;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // Spawn a component that we will update with monitor information
    commands.spawn((
        TextBundle::from_section("Monitor Info: loading…", default()),
        MonitorInfoText,
    ));
}

fn text_update_system(
    mut info_block: Query<(&mut Text,), With<MonitorInfoText>>,
    monitors: Query<(&Monitor,)>,
) {
    let (mut text,) = info_block.single_mut();

    let mut info = Vec::new();
    for (monitor,) in monitors.iter() {
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
    }
    text.sections = info;
}
