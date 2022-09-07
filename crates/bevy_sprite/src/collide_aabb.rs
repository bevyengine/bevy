//! Utilities for detecting if and on which side two axis-aligned bounding boxes (AABB) collide.

use bevy_math::{Vec2, Vec3};

#[derive(Debug, PartialEq, Eq)]
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
///
/// The collision side is determined based on the position of the second rectangle "B":
/// * `Left` means left of "B", `Top` means top of "B", ...
/// * If the collision appears on multiple sides then the side which stretches the farthest is returned
/// unless all sides are involved (`Inside`)
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

#[cfg(test)]
mod test {
    use super::*;

    fn collide_two_rectangles(
        // (x, y, size x, size y)
        a: (f32, f32, f32, f32),
        b: (f32, f32, f32, f32),
    ) -> Option<Collision> {
        collide(
            Vec3::new(a.0, a.1, 0.),
            Vec2::new(a.2, a.3),
            Vec3::new(b.0, b.1, 0.),
            Vec2::new(b.2, b.3),
        )
    }

    #[test]
    fn inside_collision() {
        // Identical
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 1., 1., 1.),
            (1., 1., 1., 1.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Inside);
        // B inside A
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 2., 2., 2.),
            (2., 2., 1., 1.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Inside);
        // A inside B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 2., 1., 1.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Inside);
    }

    #[test]
    fn collision_based_on_b() {
        // Right of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 2., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Right);
        // Left of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 2., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Left);
        // Top of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 3., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Top);
        // Bottom of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 1., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Bottom);
    }

    // In case the X-collision depth is equal to the Y-collision depth, always
    // prefer X-collision, meaning, `Left` or `Right` over `Top` and `Bottom`.
    #[test]
    fn prefer_x_collision() {
        // Bottom-left collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 1., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Left);
        // Top-left collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 3., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Left);
        // Bottom-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 1., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Right);
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2., 2., 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Right);
    }

    // If the collision intersection area stretches more along the Y-axis then
    // return `Top` or `Bottom`. Otherwise, `Left` or `Right`.
    #[test]
    fn collision_depth_wins() {
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2.5, 2.,2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Top);
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2., 2.5, 2., 2.),
        ).expect("Collision expected");
        assert_eq!(res, Collision::Right);
    }
}
