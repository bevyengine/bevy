use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(touch_event_system.system())
        .run();
}

fn touch_event_system(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.iter() {
        match event.phase {
            TouchPhase::Started => info!("Touch started."),
            TouchPhase::Moved => info!("Touch moved."),
            TouchPhase::Ended => info!("Touch ended."),
            TouchPhase::Cancelled => info!("Touch cancelled.")
        }
        info!("Touched at ({}, {})", event.position.x, event.position.y);
        info!("Finger: {}", event.id);
        if let Some(force) = event.force {
            match force {
                ForceTouch::Calibrated { force, max_possible_force, altitude_angle } => {
                    info!("Pressed with force of {}/{}, with altitude of {}", force, max_possible_force, altitude_angle.unwrap_or(0.0));
                }
                ForceTouch::Normalized(force ) => {
                    info!("Pressed with force of {}", force)
                }
            }
        }
    }
}
