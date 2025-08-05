//! Demo of the Remote Inspector Plugin
//!
//! This example shows how to use the remote inspector plugin to inspect entities
//! in a running Bevy application remotely.
//!
//! Run this example, then run a separate inspector application to connect to it.

use bevy::prelude::*;

fn main() {
    println!("=== REMOTE INSPECTOR DEMO ===");
    println!("This demo shows how to integrate the remote inspector plugin.");
    println!("For now, this is a placeholder until the full integration is complete.");
    println!("");
    println!("To use the remote inspector:");
    println!("1. Add bevy_remote to your target app:");
    println!("   App::new()");
    println!("       .add_plugins(DefaultPlugins)");
    println!("       .add_plugins(bevy::remote::RemotePlugin::default())");
    println!("       .run();");
    println!("");
    println!("2. Run the standalone inspector (from the original bevy_remote_inspector):");
    println!("   cargo run --bin bevy_remote_inspector");
    println!("");
    println!("Once the full integration is complete, you'll be able to add:");
    println!("   .add_plugins(bevy_dev_tools::inspector::InspectorPlugin)");
    
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Inspector Demo - Target Application".to_string(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_systems(Startup, setup_demo)
        .add_systems(Update, update_demo)
        .run();
}

#[derive(Component, Reflect)]
#[reflect(Component)]
struct DemoComponent {
    value: f32,
    name: String,
}

fn setup_demo(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2d);
    
    // Demo entity that changes over time
    commands.spawn((
        DemoComponent {
            value: 0.0,
            name: "Demo Entity".to_string(),
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("DemoEntity"),
    ));
    
    println!("Demo entity created - connect with remote inspector to view it!");
    println!("bevy_remote available at: http://localhost:15702");
}

fn update_demo(
    time: Res<Time>,
    mut query: Query<&mut DemoComponent>,
) {
    for mut demo in query.iter_mut() {
        demo.value = time.elapsed_secs().sin() * 100.0;
    }
}