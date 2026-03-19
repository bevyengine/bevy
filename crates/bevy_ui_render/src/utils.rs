use bevy_math::Vec2;

#[inline]
pub(crate) fn pack_uv(uv: Vec2) -> [u16; 2] {
    [
        (uv.x.clamp(0.0, 1.0) * 65535.0).round() as u16,
        (uv.y.clamp(0.0, 1.0) * 65535.0).round() as u16,
    ]
}
