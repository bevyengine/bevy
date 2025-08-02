//! Test application for the new robust virtual scrolling implementation

use bevy::prelude::*;
use bevy_remote_inspector::InspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(InspectorPlugin)
        .add_systems(Startup, setup_test_entities)
        .run();
}

/// Create some test entities to populate the inspector
fn setup_test_entities(mut commands: Commands) {
    // Create a variety of test entities to test virtual scrolling
    for i in 0..100 {
        let mut entity_commands = commands.spawn(Transform::from_xyz(i as f32, 0.0, 0.0));
        
        // Add different components based on index to create variety
        match i % 5 {
            0 => {
                entity_commands.insert(Name::new(format!("Player_{}", i)));
            }
            1 => {
                entity_commands.insert(Name::new(format!("Enemy_{}", i)));
            }
            2 => {
                entity_commands.insert(Name::new(format!("Item_{}", i)));
            }
            3 => {
                entity_commands.insert(Name::new(format!("Building_{}", i)));
            }
            _ => {
                entity_commands.insert(Name::new(format!("Entity_{}", i)));
            }
        }
        
        // Add some random components for variety
        if i % 3 == 0 {
            entity_commands.insert(Visibility::default());
        }
        
        if i % 4 == 0 {
            entity_commands.insert(GlobalTransform::default());
        }
    }
    
    println!("✅ Created 100 test entities for virtual scrolling test");
}