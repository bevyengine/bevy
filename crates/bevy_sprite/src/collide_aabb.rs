//! Utilities for detecting if and on which side two axis-aligned bounding boxes (AABB) collide.

use bevy_math::{Vec2, Vec3};

#[derive(Debug)]
pub enum Collision {
    Left,
    Right,
    Top,
    Bottom,
    Inside,
}

// TODO: ideally we can remove this once bevy gets a physics system
/// Axis-aligned bounding box collision with "side" detection
/// * `a_pos` and `b_pos` are the center positions of the rectangles, typically obtained by
/// extracting the `translation` field from a `Transform` component
/// * `a_size` and `b_size` are the dimensions (width and height) of the rectangles.
pub fn collide(a_pos: Vec3, a_size: Vec2, b_pos: Vec3, b_size: Vec2) -> Option<Collision> {
    let a_min = a_pos.truncate() - a_size / 2.0;
    let a_max = a_pos.truncate() + a_size / 2.0;

    let b_min = b_pos.truncate() - b_size / 2.0;
    let b_max = b_pos.truncate() + b_size / 2.0;

    // check to see if the two rectangles are intersecting
    if a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y {
        // check to see if we hit on the left or right side
        let (x_collision, x_depth) = if a_min.x < b_min.x && a_max.x > b_min.x && a_max.x < b_max.x
        {
            (Collision::Left, b_min.x - a_max.x)
        } else if a_min.x > b_min.x && a_min.x < b_max.x && a_max.x > b_max.x {
            (Collision::Right, a_min.x - b_max.x)
        } else {
            (Collision::Inside, -f32::INFINITY)
        };

        // check to see if we hit on the top or bottom side
        let (y_collision, y_depth) = if a_min.y < b_min.y && a_max.y > b_min.y && a_max.y < b_max.y
        {
            (Collision::Bottom, b_min.y - a_max.y)
        } else if a_min.y > b_min.y && a_min.y < b_max.y && a_max.y > b_max.y {
            (Collision::Top, a_min.y - b_max.y)
        } else {
            (Collision::Inside, -f32::INFINITY)
        };

        // if we had an "x" and a "y" collision, pick the "primary" side using penetration depth
        if y_depth.abs() < x_depth.abs() {
            Some(y_collision)
        } else {
            Some(x_collision)
        }
    } else {
        None
    }
}
