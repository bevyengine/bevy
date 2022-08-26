use std::borrow::Cow;

/// Provide the name of the type as string.
pub trait TypeName {
    fn name() -> Cow<'static, str>;
}

/// A object-safe version of [`TypeName`].
/// Automatically implemented via blanket implementation.
pub trait ReflectTypeName {
    // FIXME: named with a trailling underscore to avoid conflict
    // with Reflect::type_name until it's replaced by this method.
    fn type_name_(&self) -> Cow<'static, str>;
}

impl<T: TypeName> ReflectTypeName for T {
    #[inline]
    fn type_name_(&self) -> Cow<'static, str> {
        Self::name()
    }
}

macro_rules! impl_type_name_tuple {
    (
        $($t:tt),*
    ) => {
        impl<$($t: TypeName),*> TypeName for ($($t,)*) {
            #[allow(non_snake_case)]
            fn name() -> Cow<'static, str> {
                $(let $t = <$t as TypeName>::name();)*
                let s = format!(
                    concat!($(impl_type_name_tuple!(@bracket $t),)*),
                    $($t,)*
                );
                Cow::Owned(s)
            }
        }
    };
    (@bracket $t:tt) => {"{}"}
}

impl_type_name_tuple!();
impl_type_name_tuple!(A);
impl_type_name_tuple!(A, B);
impl_type_name_tuple!(A, B, C);
impl_type_name_tuple!(A, B, C, D);
impl_type_name_tuple!(A, B, C, D, E);
impl_type_name_tuple!(A, B, C, D, E, F);
impl_type_name_tuple!(A, B, C, D, E, F, G);
impl_type_name_tuple!(A, B, C, D, E, F, G, H);
impl_type_name_tuple!(A, B, C, D, E, F, G, H, I);
impl_type_name_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_type_name_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_type_name_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
