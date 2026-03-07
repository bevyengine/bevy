//! Shows how the relationship between Windows and Monitors can be used to find which monitor a
//! window is on.
use bevy::prelude::*;
use bevy::window::OnMonitor;
use bevy::window::{Monitor, PrimaryWindow};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_monitor)
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);

    // UI
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::all(px(5)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.75)),
        GlobalZIndex(i32::MAX),
        children![(
            Text::default(),
            children![TextSpan::new("Current Monitor: Unknown",),]
        )],
    ));
}

fn update_monitor(
    primary_window: Single<&OnMonitor, With<PrimaryWindow>>,
    monitors: Query<(Entity, &Monitor)>,
    example_text: Query<Entity, With<Text>>,
    mut writer: TextUiWriter,
) -> Result {
    if let Some(current_monitor) = monitors
        .iter()
        .find(|(e, ..)| *e == primary_window.0)
        .unwrap()
        .1
        .name
        .clone()
    {
        *writer.text(example_text.single()?, 1) = format!("Current Monitor: {:?}", current_monitor);
    } else {
        *writer.text(example_text.single()?, 1) = "Current Monitor: Unknown".to_string();
    }
    Ok(())
}
