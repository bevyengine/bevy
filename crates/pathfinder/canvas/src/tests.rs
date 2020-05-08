// pathfinder/canvas/src/tests.rs
//
// For this file only, any copyright is dedicated to the Public Domain.
// https://creativecommons.org/publicdomain/zero/1.0/

use pathfinder_geometry::vector::{Vector2F, vec2f};
use super::Path2D;

#[test]
pub fn test_path2d_formatting() {
    let mut path = Path2D::new();
    path.move_to(vec2f(0.0, 1.0));
    path.line_to(vec2f(2.0, 3.0));
    assert_eq!(format!("{:?}", path), "M 0 1 L 2 3");
    path.line_to(vec2f(4.0, 5.0));
    assert_eq!(format!("{:?}", path), "M 0 1 L 2 3 L 4 5");
    path.close_path();
    assert_eq!(format!("{:?}", path), "M 0 1 L 2 3 L 4 5 z");
}
