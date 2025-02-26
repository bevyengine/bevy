use bevy::math::Rect;
use bevy::math::Vec2;
use bevy::ui::LinearGradient;

fn main() {
    let linear_gradient = LinearGradient { angle: 0. };

    let start =
        linear_gradient.compute_start_point(Rect::from_corners(Vec2::ZERO, Vec2::new(200., 100.)));

    println!("start: {}", start);
}
