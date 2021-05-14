use bevy_math::{Vec3, Quat, Mat4};

#[derive(Debug)]
pub struct XRViewTransform {
    translation: Vec3,
    rotation: Quat,
}

impl XRViewTransform {
    pub fn new(translation: Vec3, rotation: Quat) -> Self {
        Self {
            translation,
            rotation
        }
    }
}

// =============================================================================
// math code adapted from
// https://github.com/KhronosGroup/OpenXR-SDK-Source/blob/master/src/common/xr_linear.h
// Copyright (c) 2017 The Khronos Group Inc.
// Copyright (c) 2016 Oculus VR, LLC.
// SPDX-License-Identifier: Apache-2.0
// =============================================================================
impl XRViewTransform {
    #[inline]
    pub fn compute_matrix(&self) -> Mat4 {
        let rotation_matrix = create_from_quaternion(self.rotation);
        let translation_matrix = create_translation(&self.translation);
        let view_matrix = translation_matrix * rotation_matrix;
        view_matrix
    }
}

// =============================================================================
// math code adapted from
// https://github.com/KhronosGroup/OpenXR-SDK-Source/blob/master/src/common/xr_linear.h
// Copyright (c) 2017 The Khronos Group Inc.
// Copyright (c) 2016 Oculus VR, LLC.
// SPDX-License-Identifier: Apache-2.0
// =============================================================================
fn create_from_quaternion(quat: Quat) -> Mat4 {
    let x2 = quat.x + quat.x;
    let y2 = quat.y + quat.y;
    let z2 = quat.z + quat.z;

    let xx2 = quat.x * x2;
    let yy2 = quat.y * y2;
    let zz2 = quat.z * z2;

    let yz2 = quat.y * z2;
    let wx2 = quat.w * x2;
    let xy2 = quat.x * y2;
    let wz2 = quat.w * z2;
    let xz2 = quat.x * z2;
    let wy2 = quat.w * y2;

    let mut cols: [f32; 16] = [0.0; 16];

    cols[0] = 1.0 - yy2 - zz2;
    cols[1] = xy2 + wz2;
    cols[2] = xz2 - wy2;
    cols[3] = 0.0;

    cols[4] = xy2 - wz2;
    cols[5] = 1.0 - xx2 - zz2;
    cols[6] = yz2 + wx2;
    cols[7] = 0.0;

    cols[8] = xz2 + wy2;
    cols[9] = yz2 - wx2;
    cols[10] = 1.0 - xx2 - yy2;
    cols[11] = 0.0;

    cols[12] = 0.0;
    cols[13] = 0.0;
    cols[14] = 0.0;
    cols[15] = 1.0;

    Mat4::from_cols_array(&cols)
}

// =============================================================================
// math code adapted from
// https://github.com/KhronosGroup/OpenXR-SDK-Source/blob/master/src/common/xr_linear.h
// Copyright (c) 2017 The Khronos Group Inc.
// Copyright (c) 2016 Oculus VR, LLC.
// SPDX-License-Identifier: Apache-2.0
// =============================================================================
fn create_translation(translation: &Vec3) -> Mat4 {
    let mut cols: [f32; 16] = [0.0; 16];
    cols[0] = 1.0;
    cols[1] = 0.0;
    cols[2] = 0.0;
    cols[3] = 0.0;
    cols[4] = 0.0;
    cols[5] = 1.0;
    cols[6] = 0.0;
    cols[7] = 0.0;
    cols[8] = 0.0;
    cols[9] = 0.0;
    cols[10] = 1.0;
    cols[11] = 0.0;
    cols[12] = translation.x;
    cols[13] = translation.y;
    cols[14] = translation.z;
    cols[15] = 1.0;
    Mat4::from_cols_array(&cols)
}


#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::{Vec3, Quat, Vec4};

    #[test]
    fn test_compute_matrix() {
        let view_transform = XRViewTransform::new(
                Vec3::new(0.013484435, 1.4237524, 0.0749487),
                Quat::from_xyzw(-0.11108862, -0.09665678, 0.0010674158, 0.9890984)
        );

        assert_eq!(view_transform.compute_matrix(), Mat4::from_cols(
            Vec4::new(0.9813127, 0.023586493, 0.19096898, 0.0),
            Vec4::new(0.019363377, 0.97531635, -0.21996151, 0.0),
            Vec4::new(-0.19144328, 0.2195488, 0.95663357, 0.0),
            Vec4::new(0.013484435, 1.4237524, 0.0749487, 1.0),
        ));

    }
}
