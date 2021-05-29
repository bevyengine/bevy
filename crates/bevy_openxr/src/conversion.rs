use bevy_xr::{XrDuration, XrTime};
use glam::{Quat, Vec2, Vec3};
use openxr as xr;

pub fn to_xr_time(time: xr::Time) -> XrTime {
    XrTime::from_nanos(time.as_nanos())
}

pub fn from_xr_time(time: XrTime) -> xr::Time {
    xr::Time::from_nanos(time.as_nanos())
}

pub fn to_xr_duration(duration: xr::Duration) -> XrDuration {
    XrDuration::from_nanos(duration.as_nanos())
}

pub fn to_vec2(v: xr::Vector2f) -> Vec2 {
    Vec2::new(v.x, v.y)
}

pub fn to_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn to_quat(q: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}
