use glam::{const_vec2, const_vec3, Vec2, Vec3};

pub trait CoordSystem2D {
    const UP: Self;
    const DOWN: Self;
    const RIGHT: Self;
    const LEFT: Self;
}

pub trait CoordSystem3D {
    const UP: Self;
    const DOWN: Self;
    const RIGHT: Self;
    const LEFT: Self;
    const FORWARD: Self;
    const BACKWARD: Self;
}

impl CoordSystem2D for Vec2 {
    const UP: Self = const_vec2!([0.0, 1.0]);
    const DOWN: Self = const_vec2!([0.0, -1.0]);
    const RIGHT: Self = const_vec2!([1.0, 0.0]);
    const LEFT: Self = const_vec2!([-1.0, 0.0]);
}

impl CoordSystem3D for Vec3 {
    const UP: Self = const_vec3!([0.0, 1.0, 0.0]);
    const DOWN: Self = const_vec3!([0.0, -1.0, 0.0]);
    const RIGHT: Self = const_vec3!([1.0, 0.0, 0.0]);
    const LEFT: Self = const_vec3!([-1.0, 0.0, 0.0]);
    const FORWARD: Self = const_vec3!([0.0, 0.0, -1.0]);
    const BACKWARD: Self = const_vec3!([0.0, 0.0, 1.0]);
}
