use bytemuck::Zeroable;

use crate::std140::{self, AsStd140};
use crate::std430::{self, AsStd430};

macro_rules! glam_vectors {
    ( $( $glam_ty:ty, $std_name:ident, ( $($field:ident),* ), )* ) => {
        $(
            impl AsStd140 for $glam_ty {
                type Std140Type = std140::$std_name;

                fn as_std140(&self) -> Self::Std140Type {
                    std140::$std_name {
                        $(
                            $field: self.$field,
                        )*
                    }
                }

                fn from_std140(value: Self::Std140Type) -> Self {
                    Self::new($(value.$field,)*)
                }
            }

            impl AsStd430 for $glam_ty {
                type Std430Type = std430::$std_name;

                fn as_std430(&self) -> Self::Std430Type {
                    std430::$std_name {
                        $(
                            $field: self.$field,
                        )*
                    }
                }

                fn from_std430(value: Self::Std430Type) -> Self {
                    Self::new($(value.$field,)*)
                }
            }
        )*
    };
}

glam_vectors! {
    glam::Vec2, Vec2, (x, y),
    glam::Vec3, Vec3, (x, y, z),
    glam::Vec4, Vec4, (x, y, z, w),
}

macro_rules! glam_matrices {
    ( $( $glam_ty:ty, $std_name:ident, ( $($glam_field:ident),* ), ( $($field:ident),* ))* ) => {
        $(
            impl AsStd140 for $glam_ty {
                type Std140Type = std140::$std_name;

                fn as_std140(&self) -> Self::Std140Type {
                    std140::$std_name {
                        $(
                            $field: self.$glam_field.as_std140(),
                        )*
                        ..Zeroable::zeroed()
                    }
                }

                fn from_std140(value: Self::Std140Type) -> Self {
                    Self::from_cols(
                        $(
                            <_ as AsStd140>::from_std140(value.$field),
                        )*
                    )
                }
            }

            impl AsStd430 for $glam_ty {
                type Std430Type = std430::$std_name;

                fn as_std430(&self) -> Self::Std430Type {
                    std430::$std_name {
                        $(
                            $field: self.$glam_field.as_std430(),
                        )*
                        ..Zeroable::zeroed()
                    }
                }

                fn from_std430(value: Self::Std430Type) -> Self {
                    Self::from_cols(
                        $(
                            <_ as AsStd430>::from_std430(value.$field),
                        )*
                    )
                }
            }
        )*
    };
}

glam_matrices! {
    glam::Mat2, Mat2, (x_axis, y_axis), (x, y)
    glam::Mat3, Mat3, (x_axis, y_axis, z_axis), (x, y, z)
    glam::Mat4, Mat4, (x_axis, y_axis, z_axis, w_axis), (x, y, z, w)
}
