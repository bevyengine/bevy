use crate::utility::GenericTypeNameCell;

/// Provide the name of the type as a string slice.
///
/// This is a stable alternative to [`std::any::type_name`] whose output isn't guarenteed
/// and may change between versions of the compiler.
///
/// This trait may be derived via [`#[derive(TypeName)]`][bevy_reflect_derive::TypeName].
///
/// ## Manual implementation
///
/// For some reason you may need to manually implement [`TypeName`].
///
/// ```ignore
/// bevy_reflect::TypeName;
///
/// struct MyType;
///
/// impl TypeName for MyType{
///     fn name() -> &'static str {
///         concat!(module_path!(), "::", "MyType")
///     }
/// }
/// ```
///
/// If your type is generic you must use
/// [`GenericTypeNameCell`][crate::utility::GenericTypeNameCell].
///
/// ```ignore
/// bevy_reflect::{TypeName, utility::GenericTypeNameCell};
///
/// struct MyType<T>(T);
///
/// impl<T: TypeName> TypeName for MyType<T> {
///     fn name() -> &'static str {
///         static CELL: GenericTypeNameCell = GenericTypeNameCell::new();
///         CELL.get_or_insert::<Self, _>(|| {
///             format!(concat!(module_path!(), "::MyType<{}>"), T::name())
///         })
///     }
/// }
/// ```
pub trait TypeName: 'static {
    /// Returns the name of the type.
    ///
    /// This is a stable alternative to [`std::any::type_name`] whose output isn't guarentee
    /// and may change between versions of the compiler.
    fn name() -> &'static str;
}

macro_rules! impl_type_name_tuple {
    (
        $($t:tt),*
    ) => {
        impl<$($t: TypeName),*> TypeName for ($($t,)*) {
            #[allow(non_snake_case)]
            fn name() -> &'static str {
                static CELL: GenericTypeNameCell = GenericTypeNameCell::new();
                CELL.get_or_insert::<Self, _>(|| {
                    $(let $t = <$t as TypeName>::name();)*
                    format!(
                        concat!("(", impl_type_name_tuple!(@bracket $($t),*), ")"),
                        $($t,)*
                    )
                })
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
