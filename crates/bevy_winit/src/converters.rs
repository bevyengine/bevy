use bevy_input::touch::{ForceTouch, TouchInput, TouchPhase};
use bevy_math::Vec2;

pub fn convert_touch_input(
    touch_input: winit::event::Touch,
    location: winit::dpi::LogicalPosition<f32>,
) -> TouchInput {
    TouchInput {
        phase: match touch_input.phase {
            winit::event::TouchPhase::Started => TouchPhase::Started,
            winit::event::TouchPhase::Moved => TouchPhase::Moved,
            winit::event::TouchPhase::Ended => TouchPhase::Ended,
            winit::event::TouchPhase::Cancelled => TouchPhase::Cancelled,
        },
        position: Vec2::new(location.x, location.y),
        force: touch_input.force.map(|f| match f {
            winit::event::Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => ForceTouch::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            },
            winit::event::Force::Normalized(x) => ForceTouch::Normalized(x),
        }),
        id: touch_input.id,
    }
}
