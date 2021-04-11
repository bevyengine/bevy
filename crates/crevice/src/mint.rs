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
            }
        )*
    };
}

mint_vectors! {
    mint::Vector2<f32>, Vec2, (x, y),
    mint::Vector3<f32>, Vec3, (x, y, z),
    mint::Vector4<f32>, Vec4, (x, y, z, w),
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
            }
        )*
    };
}

mint_matrices! {
    mint::ColumnMatrix2<f32>, Mat2, (x, y),
    mint::ColumnMatrix3<f32>, Mat3, (x, y, z),
    mint::ColumnMatrix4<f32>, Mat4, (x, y, z, w),
}
