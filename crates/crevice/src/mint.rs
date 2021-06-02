use bytemuck::Zeroable;

use crate::std140::{self, AsStd140};
use crate::std430::{self, AsStd430};

macro_rules! mint_vectors {
    ( $( $mint_ty:ty, $std_name:ident, ( $($field:ident),* ), )* ) => {
        $(
            impl AsStd140 for $mint_ty {
                type Std140Type = std140::$std_name;

                fn as_std140(&self) -> Self::Std140Type {
                    std140::$std_name {
                        $(
                            $field: self.$field,
                        )*
                    }
                }

                fn from_std140(value: Self::Std140Type) -> Self {
                    Self {
                        $(
                            $field: value.$field,
                        )*
                    }
                }
            }

            impl AsStd430 for $mint_ty {
                type Std430Type = std430::$std_name;

                fn as_std430(&self) -> Self::Std430Type {
                    std430::$std_name {
                        $(
                            $field: self.$field,
                        )*
                    }
                }

                fn from_std430(value: Self::Std430Type) -> Self {
                    Self {
                        $(
                            $field: value.$field,
                        )*
                    }
                }
            }
        )*
    };
}

mint_vectors! {
    mint::Vector2<f32>, Vec2, (x, y),
    mint::Vector3<f32>, Vec3, (x, y, z),
    mint::Vector4<f32>, Vec4, (x, y, z, w),

    mint::Vector2<i32>, IVec2, (x, y),
    mint::Vector3<i32>, IVec3, (x, y, z),
    mint::Vector4<i32>, IVec4, (x, y, z, w),

    mint::Vector2<u32>, UVec2, (x, y),
    mint::Vector3<u32>, UVec3, (x, y, z),
    mint::Vector4<u32>, UVec4, (x, y, z, w),

    mint::Vector2<bool>, BVec2, (x, y),
    mint::Vector3<bool>, BVec3, (x, y, z),
    mint::Vector4<bool>, BVec4, (x, y, z, w),

    mint::Vector2<f64>, DVec2, (x, y),
    mint::Vector3<f64>, DVec3, (x, y, z),
    mint::Vector4<f64>, DVec4, (x, y, z, w),
}

macro_rules! mint_matrices {
    ( $( $mint_ty:ty, $std_name:ident, ( $($field:ident),* ), )* ) => {
        $(
            impl AsStd140 for $mint_ty {
                type Std140Type = std140::$std_name;

                fn as_std140(&self) -> Self::Std140Type {
                    std140::$std_name {
                        $(
                            $field: self.$field.as_std140(),
                        )*
                        ..Zeroable::zeroed()
                    }
                }

                fn from_std140(value: Self::Std140Type) -> Self {
                    Self {
                        $(
                            $field: <_ as AsStd140>::from_std140(value.$field),
                        )*
                    }
                }
            }

            impl AsStd430 for $mint_ty {
                type Std430Type = std430::$std_name;

                fn as_std430(&self) -> Self::Std430Type {
                    std430::$std_name {
                        $(
                            $field: self.$field.as_std430(),
                        )*
                        ..Zeroable::zeroed()
                    }
                }

                fn from_std430(value: Self::Std430Type) -> Self {
                    Self {
                        $(
                            $field: <_ as AsStd430>::from_std430(value.$field),
                        )*
                    }
                }
            }
        )*
    };
}

mint_matrices! {
    mint::ColumnMatrix2<f32>, Mat2, (x, y),
    mint::ColumnMatrix3<f32>, Mat3, (x, y, z),
    mint::ColumnMatrix4<f32>, Mat4, (x, y, z, w),

    mint::ColumnMatrix2<f64>, DMat2, (x, y),
    mint::ColumnMatrix3<f64>, DMat3, (x, y, z),
    mint::ColumnMatrix4<f64>, DMat4, (x, y, z, w),
}
