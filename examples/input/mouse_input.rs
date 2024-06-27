//! Prints mouse button events.

use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<CurrentMouseMotion>()
        .init_resource::<CurrentMouseScroll>()
        .add_systems(Update, (mouse_click_system, mouse_move_system))
        .run();
}

#[derive(Resource, Default)]
struct CurrentMouseMotion {
    pub delta: Vec2,
}

#[derive(Resource, Default)]
struct CurrentMouseScroll {
    pub delta: Vec2,
}

// This system prints messages when you press or release the left mouse button:
fn mouse_click_system(mouse_button_input: Res<ButtonInput<MouseButton>>) {
    if mouse_button_input.pressed(MouseButton::Left) {
        info!("left mouse currently pressed");
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        info!("left mouse just pressed");
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        info!("left mouse just released");
    }
}

// This system prints messages when you press or release the left mouse button:
fn mouse_move_system(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
    mut current_mouse_motion: ResMut<CurrentMouseMotion>,
    mut current_mouse_scroll: ResMut<CurrentMouseScroll>,
) {
    if accumulated_mouse_motion.delta == Vec2::ZERO && current_mouse_motion.delta != Vec2::ZERO {
        let delta = current_mouse_motion.delta;
        info!("mouse moved ({}, {})", delta.x, delta.y);
        current_mouse_motion.delta = Vec2::ZERO;
    } else {
        current_mouse_motion.delta += accumulated_mouse_motion.delta;
    }
    if accumulated_mouse_scroll.delta == Vec2::ZERO && current_mouse_scroll.delta != Vec2::ZERO {
        let delta = current_mouse_scroll.delta;
        info!("mouse scrolled ({}, {})", delta.x, delta.y);
        current_mouse_scroll.delta = Vec2::ZERO;
    } else {
        current_mouse_scroll.delta += accumulated_mouse_scroll.delta;
    }
}
