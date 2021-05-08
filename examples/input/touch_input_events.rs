use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(touch_event_system.system())
        .run();
}

fn touch_event_system(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.iter() {
        let phase: TouchPhase = event.phase;
        let position: Vec2 = event.position;
        let id: u64 = event.id;
        let force = event.force;
        match phase {
            TouchPhase::Started => info!("Touch started."),
            TouchPhase::Moved => info!("Touch moved."),
            TouchPhase::Ended => info!("Touch ended."),
            TouchPhase::Cancelled => info!("Touch cancelled.")
        }
        info!("Touched at ({}, {})", position.x, position.y);
        info!("Finger: {}", id);
        if let Some(force) = force {
            match force {
                ForceTouch::Calibrated { force, max_possible_force, altitude_angle } => {
                    // force and max_possible_force are both f64s, while altitude_angle is an Option<f64>.
                    info!("Pressed with force of {}/{}, with altitude of {}", force, max_possible_force, altitude_angle.unwrap_or(0.0));
                }
                ForceTouch::Normalized(force ) => {
                    // force is a f64
                    info!("Pressed with force of {}", force)
                }
            }
        }
    }
}
