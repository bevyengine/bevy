use std::borrow::Cow;

/// Provide the name of the type as string.
pub trait TypeName {
    fn name() -> Cow<'static, str>;
}

/// An object-safe version of [`TypeName`].
/// Automatically implemented via blanket implementation.
pub trait ReflectTypeName {
    fn type_name(&self) -> Cow<str>;
}

impl<T: TypeName> ReflectTypeName for T {
    #[inline]
    fn type_name(&self) -> Cow<str> {
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
                    concat!("(", impl_type_name_tuple!(@bracket $($t),*), ")"),
                    $($t,)*
                );
                Cow::Owned(s)
            }
        }
    };
    (@bracket $t:tt, $($rest:tt),*) => {concat!("{}, ", impl_type_name_tuple!(@bracket $($rest),*))};
    (@bracket $t:tt) => {"{}"};
    (@bracket) => {""};
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

#[cfg(test)]
mod tests {
    use crate::{self as bevy_reflect, TypeName};

    #[test]
    fn tuple_name() {
        #[derive(TypeName)]
        #[type_name("Foo")]
        struct Foo;

        #[derive(TypeName)]
        #[type_name("Goo")]
        struct Goo;

        #[derive(TypeName)]
        #[type_name("Hoo")]
        struct Hoo;

        let s = <(Foo, Goo, Hoo) as TypeName>::name();
        assert_eq!(s, "(Foo, Goo, Hoo)");
    }
}
