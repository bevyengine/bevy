use crate::{Vec4, Quat};


pub trait RotationYawPitchRoll {
    fn rotation_ypr(yaw : f32, pitch : f32, roll : f32) -> Quat;
}
impl RotationYawPitchRoll for Quat{
    fn rotation_ypr(yaw: f32, pitch: f32, roll: f32) -> Quat {
        let half_yaw = yaw * 0.5;
        let half_pitch = pitch * 0.5;
        let half_roll = roll * 0.5;
    
        let sin_yaw = half_yaw.sin();
        let cos_yaw = half_yaw.cos();
        let sin_pitch = half_pitch.sin();
        let cos_pitch = half_pitch.cos();
        let sin_roll = half_roll.sin();
        let cos_roll = half_roll.cos();
    
        let cos_yawpitch = cos_yaw * cos_pitch;
        let sin_yawpitch = sin_yaw * sin_pitch;
    
        Quat::from_vec4(
            Vec4::new(
                (cos_yaw * sin_pitch * cos_roll) + (sin_yaw * cos_pitch * sin_roll),
                (sin_yaw * cos_pitch * cos_roll) - (cos_yaw * sin_pitch * sin_roll),
                (cos_yawpitch * sin_roll) - (sin_yawpitch * cos_roll),
                (cos_yawpitch * cos_roll) + (sin_yawpitch * sin_roll),
            )
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rotation_ypr() {
        use crate::{Quat, Vec3,  RotationYawPitchRoll};

        assert_eq!(Quat::rotation_ypr(0.0,0.0,0.0), Quat::IDENTITY);
        assert_eq!(Quat::rotation_ypr(180.0,0.0,0.0), Quat::from_rotation_y(180.0));

    }
}

