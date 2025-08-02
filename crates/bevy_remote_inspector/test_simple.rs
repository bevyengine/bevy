//! Simplified test to check if basic UI is working

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Simple UI Test".to_string(),
                resolution: (800.0, 600.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup_test_ui)
        .run();
}

fn setup_test_ui(mut commands: Commands) {
    // Spawn UI camera
    commands.spawn(Camera2d);
    
    // Root container
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            ..default()
        },
        BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    )).with_children(|parent| {
        // Left panel
        parent.spawn((
            Node {
                width: Val::Percent(30.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Entity List"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            
            // Test entity items
            for i in 1..=4 {
                parent.spawn((
                    Text::new(format!("Entity {}", i)),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    Node {
                        margin: UiRect::top(Val::Px(8.0)),
                        ..default()
                    },
                ));
            }
        });
        
        // Right panel
        parent.spawn((
            Node {
                width: Val::Percent(70.0),
                height: Val::Percent(100.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
        )).with_children(|parent| {
            parent.spawn((
                Text::new("Component Viewer"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            
            parent.spawn((
                Text::new("Transform Component:\n  translation: (0.0, 0.0, 0.0)\n  rotation: (0.0, 0.0, 0.0, 1.0)\n  scale: (1.0, 1.0, 1.0)"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.7, 0.7)),
                Node {
                    margin: UiRect::top(Val::Px(16.0)),
                    ..default()
                },
            ));
        });
    });
    
    println!("Simple test UI created!");
}