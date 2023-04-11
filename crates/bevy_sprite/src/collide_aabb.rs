//! Utilities for detecting if and on which side two axis-aligned bounding boxes (AABB) collide.

use bevy_math::{Vec2, Vec3};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
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
/// The return value is the side of `B` that `A` has collided with. `Left` means that
/// `A` collided with `B`'s left side. `Top` means that `A` collided with `B`'s top side.
/// If the collision occurs on multiple sides, the side with the deepest penetration is returned.
/// If all sides are involved, `Inside` is returned.
pub fn collide(a_pos: Vec3, a_size: Vec2, b_pos: Vec3, b_size: Vec2) -> Option<Collision> {
    let Vec2 { x: xa, y: ya } = a_pos.truncate(); // x and y of center of a
    let Vec2 { x: wa, y: ha } = a_size * 0.5; // half width and height of a
    let Vec2 { x: xb, y: yb } = b_pos.truncate();
    let Vec2 { x: wb, y: hb } = b_size * 0.5;
    let dis_x = (xa - xb).abs();
    let dis_y = (ya - yb).abs();

    // this method is just like checking the relative position of the circles

    if dis_x >= (wa + wb).abs() || dis_y >= (ha + hb).abs() {
        return None; // a is separated from b or is tangent to b
    }

    let (x_collision, x_depth) = if dis_x <= (wa - wb).abs() {
        (Collision::Inside, 0.0)
    } else if xa < xb {
        (Collision::Left, (xa + wa) - (xb - wb)) // right_a - left_b
    } else {
        (Collision::Right, (xb + wb) - (xa - wa)) // right_b - left_a
    };

    let (y_collision, y_depth) = if dis_y <= (ha - hb).abs() {
        (Collision::Inside, 0.0)
    } else if ya < yb {
        (Collision::Bottom, (ya + ha) - (yb - hb)) // top_a - bottom_b
    } else {
        (Collision::Top, (yb + hb) - (ya - ha)) // top_b - bottom_a
    };

    // choose deepest
    if y_depth > x_depth {
        Some(y_collision)
    } else {
        Some(x_collision)
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
        );
        assert_eq!(res, Some(Collision::Inside));
        // B inside A
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 2., 2., 2.),
            (2., 2., 1., 1.),
        );
        assert_eq!(res, Some(Collision::Inside));
        // A inside B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 2., 1., 1.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Inside));
    }

    #[test]
    fn collision_based_on_b() {
        // Right of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 2., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Right));
        // Left of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 2., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Left));
        // Top of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 3., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Top));
        // Bottom of B
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (2., 1., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Bottom));
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
        );
        assert_eq!(res, Some(Collision::Left));
        // Top-left collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (1., 3., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Left));
        // Bottom-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 1., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Right));
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2., 2., 2., 2.),
        );
        assert_eq!(res, Some(Collision::Right));
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
        );
        assert_eq!(res, Some(Collision::Right));
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2., 2.5, 2., 2.),
        );
        assert_eq!(res, Some(Collision::Top));
    }
}
