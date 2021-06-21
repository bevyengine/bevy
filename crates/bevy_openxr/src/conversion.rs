use bevy_math::{Quat, Vec3};
use bevy_utils::Duration;
use openxr as xr;

use crate::OpenXrTime;

pub fn to_xr_time(time: xr::Time) -> OpenXrTime {
    OpenXrTime::from_nanos(time.as_nanos() as _)
}

pub fn from_xr_time(time: OpenXrTime) -> xr::Time {
    xr::Time::from_nanos(time.as_nanos() as _)
}

pub fn to_duration(duration: xr::Duration) -> Duration {
    Duration::from_nanos(duration.as_nanos() as _)
}

pub fn from_duration(duration: Duration) -> xr::Duration {
    xr::Duration::from_nanos(duration.as_nanos() as _)
}

pub fn to_vec3(v: xr::Vector3f) -> Vec3 {
    Vec3::new(v.x, v.y, v.z)
}

pub fn to_quat(q: xr::Quaternionf) -> Quat {
    Quat::from_xyzw(q.x, q.y, q.z, q.w)
}
