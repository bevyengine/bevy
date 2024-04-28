pub use error::*;
pub use function::*;
pub use info::*;
pub use into::*;

pub mod args;
mod error;
mod function;
mod info;
mod into;
mod utils;

#[cfg(test)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::func::args::{ArgError, ArgId, ArgList, Ownership};
    use crate::{Reflect, TypePath};
    use alloc::borrow::Cow;

    #[test]
    fn should_create_dynamic_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let mut func = add.into_function();
        let args = ArgList::default().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_closure() {
        let mut func = (|a: i32, b: i32| a + b).into_function();
        let args = ArgList::default().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_create_dynamic_method() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Foo(i32);

        impl Foo {
            pub fn add(&self, other: &Foo) -> Foo {
                Foo(self.0 + other.0)
            }
        }

        crate::func::args::impl_from_arg!(Foo);

        let foo_a = Foo(25);
        let foo_b = Foo(75);

        let mut func = Foo::add.into_function();
        let args = ArgList::default().push_ref(&foo_a).push_ref(&foo_b);
        let result = func.call(args).unwrap().unwrap();
        assert_eq!(result.downcast_ref::<Foo>(), Some(&Foo(100)));
    }

    #[test]
    fn should_allow_zero_args() {
        fn foo() -> String {
            String::from("Hello, World!")
        }

        let mut func = foo.into_function();
        let args = ArgList::default();
        let result = func.call(args).unwrap().unwrap();
        assert_eq!(
            result.downcast_ref::<String>(),
            Some(&String::from("Hello, World!"))
        );
    }

    #[test]
    fn should_allow_unit_return() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::default().push_owned(123_i32);
        let result = func.call(args).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn should_error_on_missing_args() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::default();
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FuncError::ArgCount {
                expected: 1,
                received: 0
            }
        );
    }

    #[test]
    fn should_error_on_too_many_args() {
        fn foo() {}

        let mut func = foo.into_function();
        let args = ArgList::default().push_owned(123_i32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FuncError::ArgCount {
                expected: 0,
                received: 1
            }
        );
    }

    #[test]
    fn should_error_on_invalid_arg_type() {
        fn foo(_: i32) {}

        let mut func = foo.into_function();
        let args = ArgList::default().push_owned(123_u32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FuncError::Arg(ArgError::UnexpectedType {
                id: ArgId::Index(0),
                expected: Cow::Borrowed(i32::type_path()),
                received: Cow::Borrowed(u32::type_path())
            })
        );
    }

    #[test]
    fn should_error_on_invalid_arg_ownership() {
        fn foo(_: &i32) {}

        let mut func = foo.into_function();
        let args = ArgList::default().push_owned(123_i32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FuncError::Arg(ArgError::InvalidOwnership {
                id: ArgId::Index(0),
                expected: Ownership::Ref,
                received: Ownership::Owned
            })
        );
    }
}
