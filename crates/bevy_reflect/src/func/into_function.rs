use std::panic::{RefUnwindSafe, UnwindSafe};

use crate::func::function::DynamicFunction;
use crate::func::{ReflectFn, TypedFunction};

/// A trait for types that can be converted into a [`DynamicFunction`].
///
/// This trait is automatically implemented for many standard Rust functions
/// that also implement [`ReflectFn`] and [`TypedFunction`].
///
/// To handle types such as closures that capture references to their environment,
/// see [`IntoClosure`] instead.
///
/// See the [module-level documentation] for more information.
///
/// [`IntoClosure`]: crate::func::IntoClosure
/// [module-level documentation]: crate::func
pub trait IntoFunction<Marker> {
    /// Converts [`Self`] into a [`DynamicFunction`].
    fn into_function(self) -> DynamicFunction;
}

impl<F, Marker1, Marker2> IntoFunction<(Marker1, Marker2)> for F
where
    F: ReflectFn<'static, Marker1>
        + TypedFunction<Marker2>
        // Ideally, we'd only implement `IntoFunction` on actual function types
        // (i.e. functions that do not capture their environment at all),
        // but this would only work if users first explicitly coerced their functions
        // to a function pointer like `(add as fn(i32, i32) -> i32).into_function()`,
        // which is certainly not the best user experience.
        // So as a compromise, we'll stick to allowing any type that implements
        // `ReflectFn` and `TypedFunction`, but also add the following trait bounds
        // that all `fn` types implement:
        + Clone
        + Copy
        + Send
        + Sync
        + Unpin
        + UnwindSafe
        + RefUnwindSafe
        + 'static,
{
    fn into_function(self) -> DynamicFunction {
        // Note that to further guarantee that `self` is a true `fn` type,
        // we could add a compile time assertion that `F` is zero-sized.
        // However, we don't do this because it would prevent users from
        // converting function pointers into `DynamicFunction`s.

        DynamicFunction::new(
            move |args, info| self.reflect_call(args, info),
            Self::function_info(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::func::ArgList;
    use bevy_reflect_derive::Reflect;

    #[test]
    fn should_create_dynamic_function_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func = add.into_function();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_function_from_function_pointer() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func = (add as fn(i32, i32) -> i32).into_function();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_function_from_anonymous_function() {
        let func = (|a: i32, b: i32| a + b).into_function();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_function_from_method() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo(i32);

        impl Foo {
            pub fn add(&self, other: &Foo) -> Foo {
                Foo(self.0 + other.0)
            }
        }

        let foo_a = Foo(25);
        let foo_b = Foo(75);

        let func = Foo::add.into_function();
        let args = ArgList::new().push_ref(&foo_a).push_ref(&foo_b);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<Foo>(), Some(&Foo(100)));
    }

    #[test]
    fn should_allow_zero_args() {
        fn foo() -> String {
            String::from("Hello, World!")
        }

        let func = foo.into_function();
        let args = ArgList::new();
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(
            result.try_downcast_ref::<String>(),
            Some(&String::from("Hello, World!"))
        );
    }

    #[test]
    fn should_allow_unit_return() {
        fn foo(_: i32) {}

        let func = foo.into_function();
        let args = ArgList::new().push_owned(123_i32);
        let result = func.call(args).unwrap();
        assert!(result.is_unit());
    }

    #[test]
    fn should_allow_reference_return() {
        fn foo<'a>(value: &'a i32, _: String, _: &bool) -> &'a i32 {
            value
        }

        let value: i32 = 123;
        let func = foo.into_function();
        let args = ArgList::new()
            .push_ref(&value)
            .push_owned(String::from("Hello, World!"))
            .push_ref(&true);
        let result = func.call(args).unwrap().unwrap_ref();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_allow_mutable_reference_return() {
        fn foo<'a>(value: &'a mut i32, _: String, _: &bool) -> &'a mut i32 {
            value
        }

        let mut value: i32 = 123;
        let func = foo.into_function();
        let args = ArgList::new()
            .push_mut(&mut value)
            .push_owned(String::from("Hello, World!"))
            .push_ref(&true);
        let result = func.call(args).unwrap().unwrap_mut();
        assert_eq!(result.try_downcast_mut::<i32>(), Some(&mut 123));
    }

    #[test]
    fn should_default_with_function_type_name() {
        fn foo() {}

        let func = foo.into_function();
        assert_eq!(
            func.info().name(),
            Some("bevy_reflect::func::into_function::tests::should_default_with_function_type_name::foo")
        );
    }
}
