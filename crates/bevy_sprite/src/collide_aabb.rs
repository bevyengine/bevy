//! Utilities for detecting if and on which side two axis-aligned bounding boxes (AABB) collide.

use bevy_math::{Vec2, Vec3};

/// The side where a collision occurred, as returned by [`collide`].
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Collision {
    Left,
    Right,
    Top,
    Bottom,
    Inside,
}

struct CollisionBox {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

impl CollisionBox {
    pub fn new(pos: Vec3, size: Vec2) -> Self {
        Self {
            top: pos.y + size.y / 2.,
            bottom: pos.y - size.y / 2.,
            left: pos.x - size.x / 2.,
            right: pos.x + size.x / 2.,
        }
    }
}

// TODO: ideally we can remove this once bevy gets a physics system
/// Axis-aligned bounding box collision with "side" detection.
///
/// The [Collision], in case it occurred, is the side of `b` where `a` hit.
///
/// * `a_pos` and `b_pos` are the center positions of the rectangles, typically obtained by
/// extracting the `translation` field from a [`Transform`](bevy_transform::components::Transform) component
/// * `a_size` and `b_size` are the dimensions (width and height) of the rectangles.
///
/// The return value is the side of `B` that `A` has collided with. [`Collision::Left`] means that
/// `A` collided with `B`'s left side. [`Collision::Top`] means that `A` collided with `B`'s top side.
/// If the collision occurs on multiple sides, the side with the shallowest penetration is returned.
/// If all sides are involved, [`Collision::Inside`] is returned.
pub fn collide(a_pos: Vec3, a_size: Vec2, b_pos: Vec3, b_size: Vec2) -> Option<Collision> {
    let a = CollisionBox::new(a_pos, a_size);
    let b = CollisionBox::new(b_pos, b_size);

    // check to see if the two rectangles are intersecting
    if a.left < b.right && a.right > b.left && a.bottom < b.top && a.top > b.bottom {
        // check to see if we hit on the left or right side
        let (x_collision, x_depth) = if a.left < b.left && a.right > b.left && a.right < b.right {
            (Collision::Left, b.left - a.right)
        } else if a.left > b.left && a.left < b.right && a.right > b.right {
            (Collision::Right, a.left - b.right)
        } else {
            (Collision::Inside, -f32::INFINITY)
        };

        // check to see if we hit on the top or bottom side
        let (y_collision, y_depth) = if a.bottom < b.bottom && a.top > b.bottom && a.top < b.top {
            (Collision::Bottom, b.bottom - a.top)
        } else if a.bottom > b.bottom && a.bottom < b.top && a.top > b.top {
            (Collision::Top, a.bottom - b.top)
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
mod tests {
    use super::*;

    ///
    ///
    ///     ______
    ///     |    |
    ///     | A  |
    /// ----|----|----
    /// |   |____|   |
    /// |     B      |
    /// |____________|
    ///
    #[test]
    fn top_collision() {
        let a = Vec3::new(0., 30., 0.);
        let b = Vec3::new(0., 0., 0.);

        check(a, b, Some(Collision::Top));
    }

    ///
    ///
    /// --------------
    /// |      B     |
    /// |    _____   |
    /// |___|____|___|
    ///     |    |
    ///     | A  |
    ///     |    |
    ///     -----
    #[test]
    fn bottom_collision() {
        let a = Vec3::new(0., -30., 0.);
        let b = Vec3::new(0., 0., 0.);

        check(a, b, Some(Collision::Bottom));
    }

    ///
    ///   ______
    ///  |   --|-----------
    ///  |   | |          |
    ///  | A | |    B     |
    ///  |   |_|__________|
    ///  |_____|
    ///
    #[test]
    fn left_collision() {
        let a = Vec3::new(0., 0., 0.);
        let b = Vec3::new(30., 0., 0.);

        check(a, b, Some(Collision::Left));
    }

    ///
    ///             ______
    /// -----------|--   |
    /// |      B   | |   |
    /// |          | | A |
    /// |__________|_|   |
    ///            |_____|
    #[test]
    fn right_collision() {
        let a = Vec3::new(0., 0., 0.);
        let b = Vec3::new(-30., 0., 0.);

        check(a, b, Some(Collision::Right));
    }

    ///
    ///     ______
    /// ----|----|----
    /// |   |    |   |
    /// |   |    | B |
    /// |___|____|___|
    ///     | A  |
    ///     |____|
    #[test]
    fn without_corners_on_intersection_area() {
        let a = Vec3::new(0., 0., 0.);
        let b = Vec3::new(0., 0., 0.);

        check(a, b, Some(Collision::Inside));
    }

    fn check(a: Vec3, b: Vec3, expected: Option<Collision>) {
        let a_size = Vec2::new(30., 50.);
        let b_size = Vec2::new(50., 30.);
        assert_eq!(collide(a, a_size, b, b_size), expected);
    }

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
        assert_eq!(res, Some(Collision::Top));
        // Top-right collision
        #[rustfmt::skip]
        let res = collide_two_rectangles(
            (3., 3., 2., 2.),
            (2., 2.5, 2., 2.),
        );
        assert_eq!(res, Some(Collision::Right));
    }
}
