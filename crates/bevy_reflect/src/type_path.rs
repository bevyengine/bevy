use crate::utility::GenericTypePathCell;

/// Provides the path of the type as a string.
///
/// This is a stable alternative to [`std::any::type_name`] whose output isn't guarenteed
/// and may change between versions of the compiler.
///
/// This trait may be derived via [`#[derive(TypePath)]`][bevy_reflect_derive::TypePath].
///
/// ## Manual implementation
///
/// If you need to manually implement [`TypePath`], it's recommended to follow
/// the example below (unless specifying a custom name).
///
/// ```ignore
/// bevy_reflect::TypePath;
///
/// struct MyType;
///
/// impl TypePath for MyType{
///     fn type_path() -> &'static str {
///         concat!(module_path!(), "::", "MyType")
///     }
/// }
/// ```
///
/// If your type is generic you must use
/// [`GenericTypePathCell`][crate::utility::GenericTypePathCell].
///
/// ```ignore
/// bevy_reflect::{TypePath, utility::GenericTypePathCell};
///
/// struct MyType<T>(T);
///
/// impl<T: TypePath> TypePath for MyType<T> {
///     fn name() -> &'static str {
///         static CELL: GenericTypePathCell = GenericTypePathCell::new();
///         CELL.get_or_insert::<Self, _>(|| {
///             format!(concat!(module_path!(), "::MyType<{}>"), T::name())
///         })
///     }
/// }
/// ```
pub trait TypePath: 'static {
    /// Returns the full path of the type.
    ///
    /// This is a stable alternative to [`std::any::type_name`] whose output isn't guarenteed
    /// and may change between versions of the compiler.
    fn type_path() -> &'static str;

    /// The short type name, without generics.
    ///
    /// e.g. `MyType`
    fn short_type_name_base() -> &'static str;

    /// The short type name, with generics.
    ///
    /// e.g. `MyType<Generics>`
    fn short_type_name() -> &'static str;

    /// The full type path, minus the actual type.
    ///
    /// e.g. `my_crate::my_mod`
    fn module_path() -> &'static str;

    /// The crate name.
    ///
    /// e.g. `"my_crate`
    fn crate_name() -> &'static str;
}

/// Returns the [type path] of `T`.
///
/// [type path]: TypePath
pub fn type_path<T: TypePath + ?Sized>() -> &'static str {
    T::type_path()
}

macro_rules! impl_type_name_tuple {
    (
        $($t:tt),*
    ) => {
        impl<$($t: TypePath),*> TypePath for ($($t,)*) {
            #[allow(non_snake_case)]
            fn type_path() -> &'static str {
                static CELL: GenericTypePathCell = GenericTypePathCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    $(let $t = <$t as TypePath>::type_path();)*
                    format!(
                        concat!("(", impl_type_name_tuple!(@bracket $($t),*), ")"),
                        $($t,)*
                    )
                })
            }

            fn short_type_name_base() -> &'static str {
                // FIXME: how to handle tuple ?
                Self::type_path()
            }

            fn short_type_name() -> &'static str {
                // FIXME: how to handle tuple ?
                Self::type_path()
            }

            fn module_path() -> &'static str {
                // FIXME: how to handle tuple ?
                Self::type_path()
            }

            fn crate_name() -> &'static str {
                // FIXME: how to handle tuple ?
                ""
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
    use crate::{self as bevy_reflect, TypePath};

    #[test]
    fn tuple_name() {
        #[derive(TypePath)]
        #[type_path(path = "")]
        struct Foo;

        #[derive(TypePath)]
        #[type_path(path = "")]
        struct Goo;

        #[derive(TypePath)]
        #[type_path(path = "")]
        struct Hoo;

        let s = <(Foo, Goo, Hoo) as TypePath>::type_path();
        assert_eq!(s, "(Foo, Goo, Hoo)");
    }
}
