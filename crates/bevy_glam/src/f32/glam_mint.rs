use super::{Mat2, Mat3, Mat4, Quat, Vec2, Vec3, Vec4};
use mint;

impl From<mint::Point2<f32>> for Vec2 {
    fn from(v: mint::Point2<f32>) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<Vec2> for mint::Point2<f32> {
    fn from(v: Vec2) -> Self {
        let (x, y) = v.into();
        Self { x, y }
    }
}

impl From<mint::Point3<f32>> for Vec3 {
    fn from(v: mint::Point3<f32>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for mint::Point3<f32> {
    fn from(v: Vec3) -> Self {
        let (x, y, z) = v.into();
        Self { x, y, z }
    }
}

impl From<mint::Vector2<f32>> for Vec2 {
    fn from(v: mint::Vector2<f32>) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<Vec2> for mint::Vector2<f32> {
    fn from(v: Vec2) -> Self {
        let (x, y) = v.into();
        Self { x, y }
    }
}

impl From<mint::Vector3<f32>> for Vec3 {
    fn from(v: mint::Vector3<f32>) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<Vec3> for mint::Vector3<f32> {
    fn from(v: Vec3) -> Self {
        let (x, y, z) = v.into();
        Self { x, y, z }
    }
}

impl From<mint::Vector4<f32>> for Vec4 {
    fn from(v: mint::Vector4<f32>) -> Self {
        Self::new(v.x, v.y, v.z, v.w)
    }
}

impl From<Vec4> for mint::Vector4<f32> {
    fn from(v: Vec4) -> Self {
        let (x, y, z, w) = v.into();
        Self { x, y, z, w }
    }
}

impl From<mint::Quaternion<f32>> for Quat {
    fn from(q: mint::Quaternion<f32>) -> Self {
        Self::from_xyzw(q.v.x, q.v.y, q.v.z, q.s)
    }
}

impl From<Quat> for mint::Quaternion<f32> {
    fn from(q: Quat) -> Self {
        let (x, y, z, s) = q.into();
        Self {
            s,
            v: mint::Vector3 { x, y, z },
        }
    }
}

impl From<mint::RowMatrix2<f32>> for Mat2 {
    fn from(m: mint::RowMatrix2<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into()).transpose()
    }
}

impl From<Mat2> for mint::RowMatrix2<f32> {
    fn from(m: Mat2) -> Self {
        let mt = m.transpose();
        Self {
            x: mt.x_axis().into(),
            y: mt.y_axis().into(),
        }
    }
}

impl From<mint::ColumnMatrix2<f32>> for Mat2 {
    fn from(m: mint::ColumnMatrix2<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into())
    }
}

impl From<Mat2> for mint::ColumnMatrix2<f32> {
    fn from(m: Mat2) -> Self {
        Self {
            x: m.x_axis().into(),
            y: m.y_axis().into(),
        }
    }
}

impl From<mint::RowMatrix3<f32>> for Mat3 {
    fn from(m: mint::RowMatrix3<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into(), m.z.into()).transpose()
    }
}

impl From<Mat3> for mint::RowMatrix3<f32> {
    fn from(m: Mat3) -> Self {
        let mt = m.transpose();
        Self {
            x: mt.x_axis().into(),
            y: mt.y_axis().into(),
            z: mt.z_axis().into(),
        }
    }
}

impl From<mint::ColumnMatrix3<f32>> for Mat3 {
    fn from(m: mint::ColumnMatrix3<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into(), m.z.into())
    }
}

impl From<Mat3> for mint::ColumnMatrix3<f32> {
    fn from(m: Mat3) -> Self {
        Self {
            x: m.x_axis().into(),
            y: m.y_axis().into(),
            z: m.z_axis().into(),
        }
    }
}

impl From<mint::RowMatrix4<f32>> for Mat4 {
    fn from(m: mint::RowMatrix4<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into(), m.z.into(), m.w.into()).transpose()
    }
}

impl From<Mat4> for mint::RowMatrix4<f32> {
    fn from(m: Mat4) -> Self {
        let mt = m.transpose();
        Self {
            x: mt.x_axis().into(),
            y: mt.y_axis().into(),
            z: mt.z_axis().into(),
            w: mt.w_axis().into(),
        }
    }
}

impl From<mint::ColumnMatrix4<f32>> for Mat4 {
    fn from(m: mint::ColumnMatrix4<f32>) -> Self {
        Self::from_cols(m.x.into(), m.y.into(), m.z.into(), m.w.into())
    }
}

impl From<Mat4> for mint::ColumnMatrix4<f32> {
    fn from(m: Mat4) -> Self {
        Self {
            x: m.x_axis().into(),
            y: m.y_axis().into(),
            z: m.z_axis().into(),
            w: m.w_axis().into(),
        }
    }
}
#[cfg(test)]
mod test {
    use mint;

    #[test]
    fn test_point2() {
        use crate::Vec2;
        let m = mint::Point2 { x: 1.0, y: 2.0 };
        let g = Vec2::from(m);
        assert_eq!(g, Vec2::new(1.0, 2.0));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_point3() {
        use crate::Vec3;
        let m = mint::Point3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let g = Vec3::from(m);
        assert_eq!(g, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_vector2() {
        use crate::Vec2;
        let m = mint::Vector2 { x: 1.0, y: 2.0 };
        let g = Vec2::from(m);
        assert_eq!(g, Vec2::new(1.0, 2.0));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_vector3() {
        use crate::Vec3;
        let m = mint::Vector3 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        };
        let g = Vec3::from(m);
        assert_eq!(g, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_vector4() {
        use crate::Vec4;
        let m = mint::Vector4 {
            x: 1.0,
            y: 2.0,
            z: 3.0,
            w: 4.0,
        };
        let g = Vec4::from(m);
        assert_eq!(g, Vec4::new(1.0, 2.0, 3.0, 4.0));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_quaternion() {
        use crate::Quat;
        let m = mint::Quaternion {
            v: mint::Vector3 {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            },
            s: 4.0,
        };
        let g = Quat::from(m);
        assert_eq!(g, Quat::from((1.0, 2.0, 3.0, 4.0)));
        assert_eq!(m, g.into());
    }

    #[test]
    fn test_matrix2() {
        use crate::Mat2;
        let g = Mat2::from_cols_array_2d(&[[1.0, 2.0], [3.0, 4.0]]);
        let m = mint::ColumnMatrix2::from(g);
        assert_eq!(g, Mat2::from(m));
        let mt = mint::RowMatrix2::from(g);
        assert_eq!(mt, mint::RowMatrix2::from([[1.0, 3.0], [2.0, 4.0]]));
        assert_eq!(g, Mat2::from(mt));
    }

    #[test]
    fn test_matrix3() {
        use crate::Mat3;
        let g = Mat3::from_cols_array_2d(&[[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]);
        let m = mint::ColumnMatrix3::from(g);
        assert_eq!(g, Mat3::from(m));
        let mt = mint::RowMatrix3::from(g);
        assert_eq!(
            mt,
            mint::RowMatrix3::from([[1.0, 4.0, 7.0], [2.0, 5.0, 8.0], [3.0, 6.0, 9.0]])
        );
        assert_eq!(g, Mat3::from(mt));
    }

    #[test]
    fn test_matrix4() {
        use crate::Mat4;
        let g = Mat4::from_cols_array_2d(&[
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ]);
        let m = mint::ColumnMatrix4::from(g);
        assert_eq!(g, Mat4::from(m));
        let mt = mint::RowMatrix4::from(g);
        assert_eq!(
            mt,
            mint::RowMatrix4::from([
                [1.0, 5.0, 9.0, 13.0],
                [2.0, 6.0, 10.0, 14.0],
                [3.0, 7.0, 11.0, 15.0],
                [4.0, 8.0, 12.0, 16.0]
            ])
        );
        assert_eq!(g, Mat4::from(mt));
    }
}
