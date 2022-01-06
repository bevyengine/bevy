#![allow(unused_macros)]

macro_rules! easy_impl {
    ( $( $std_name:ident $imp_ty:ty { $($field:ident),* }, )* ) => {
        $(
            impl crate::std140::AsStd140 for $imp_ty {
                type Output = crate::std140::$std_name;

                #[inline]
                fn as_std140(&self) -> Self::Output {
                    crate::std140::$std_name {
                        $(
                            $field: self.$field.as_std140(),
                        )*
                        ..bytemuck::Zeroable::zeroed()
                    }
                }

                #[inline]
                fn from_std140(value: Self::Output) -> Self {
                    Self {
                        $(
                            $field: <_ as crate::std140::AsStd140>::from_std140(value.$field),
                        )*
                    }
                }
            }

            impl crate::std430::AsStd430 for $imp_ty {
                type Output = crate::std430::$std_name;

                #[inline]
                fn as_std430(&self) -> Self::Output {
                    crate::std430::$std_name {
                        $(
                            $field: self.$field.as_std430(),
                        )*
                        ..bytemuck::Zeroable::zeroed()
                    }
                }

                #[inline]
                fn from_std430(value: Self::Output) -> Self {
                    Self {
                        $(
                            $field: <_ as crate::std430::AsStd430>::from_std430(value.$field),
                        )*
                    }
                }
            }

            unsafe impl crate::glsl::Glsl for $imp_ty {
                const NAME: &'static str = crate::std140::$std_name::NAME;
            }
        )*
    };
}

macro_rules! minty_impl {
    ( $( $mint_ty:ty => $imp_ty:ty, )* ) => {
        $(
            impl crate::std140::AsStd140 for $imp_ty {
                type Output = <$mint_ty as crate::std140::AsStd140>::Output;

                #[inline]
                fn as_std140(&self) -> Self::Output {
                    let mint: $mint_ty = (*self).into();
                    mint.as_std140()
                }

                #[inline]
                fn from_std140(value: Self::Output) -> Self {
                    <$mint_ty>::from_std140(value).into()
                }
            }

            impl crate::std430::AsStd430 for $imp_ty {
                type Output = <$mint_ty as crate::std430::AsStd430>::Output;

                #[inline]
                fn as_std430(&self) -> Self::Output {
                    let mint: $mint_ty = (*self).into();
                    mint.as_std430()
                }

                #[inline]
                fn from_std430(value: Self::Output) -> Self {
                    <$mint_ty>::from_std430(value).into()
                }
            }

            unsafe impl crate::glsl::Glsl for $imp_ty {
                const NAME: &'static str = <$mint_ty>::NAME;
            }
        )*
    };
}
