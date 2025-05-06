/// A helper macro for downcasting a [`PartialReflect`] value to a concrete type.
///
/// # Syntax
///
/// The first argument to the macro is the identifier of the variable holding the reflected value.
/// This is also the default binding for all downcasted values.
///
/// All other arguments to the macro are match-like cases statements that follow the following pattern:
///
/// ```text
/// <BINDING?> <REF?> <TYPE> => <EXPR>,
/// ```
///
/// Where `<REF?>` denotes what kind of downcasting to perform:
/// - `&` - Downcasts with `try_downcast_ref`
/// - `&mut` - Downcasts with `try_downcast_mut`
/// - None - Downcasts with `try_take`
///
/// And `<BINDING?>` is an optional binding (i.e. `<IDENT> @`) for the downcasted value.
///
/// If the `<EXPR>` doesn't evaluate to `()`, an `else` case is required:
///
/// ```text
/// <BINDING?> else => <EXPR>,
/// ```
///
/// The `<BINDING?>` on an `else` case can optionally be used to access a slice that contains the
/// [`Type`] of each `<TYPE>` in the macro.
/// This can be used as a convenience for debug messages or logging.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// # use bevy_reflect::macros::select_ty;
/// # use bevy_reflect::PartialReflect;
/// #
/// fn try_to_f32(value: &dyn PartialReflect) -> Option<f32> {
///   select_ty! {value,
///     &f32 => Some(*value),
///     &i32 => Some(*value as f32),
///     else => None
///   }
/// }
/// #
/// # assert_eq!(try_to_f32(&123_i32), Some(123_f32));
/// # assert_eq!(try_to_f32(&123.0_f32), Some(123.0_f32));
/// # assert_eq!(try_to_f32(&123_u32), None);
/// ```
///
/// With bindings:
///
/// ```
/// # use bevy_reflect::macros::select_ty;
/// # use bevy_reflect::PartialReflect;
/// #
/// fn try_push_value(container: &mut dyn PartialReflect, value: i32) {
///   select_ty! {container,
///     // By default, cases use the given identifier as the binding identifier
///     &mut Vec<i32> => {
///       container.push(value);
///     },
///     // But you can also provide your own binding identifier
///     list @ &mut Vec<u32> => {
///       list.push(value as u32);
///     },
///     // The `else` case also supports bindings.
///     // Here, `types` contains all the types from the cases above
///     types @ else => panic!("expected types: {:?}", types)
///   }
/// }
/// #
/// # let mut list: Vec<i32> = vec![1, 2];
/// # try_push_value(&mut list, 3);
/// # assert_eq!(list, vec![1, 2, 3]);
/// #
/// # let mut list: Vec<u32> = vec![1, 2];
/// # try_push_value(&mut list, 3);
/// # assert_eq!(list, vec![1, 2, 3]);
/// ```
///
/// [`PartialReflect`]: crate::PartialReflect
/// [`Type`]: crate::Type
#[macro_export]
macro_rules! select_ty {

    {$value:ident} => {};

    {$value:ident, $($tt:tt)*} => {{
        // We use an import over fully-qualified syntax so users don't have to
        // cast to `dyn PartialReflect` or dereference manually
        use $crate::PartialReflect;

        select_ty!(@selector[] $value, $value, $($tt)*)
    }};

    // === Internal === //
    // Each internal selector contains:
    // 1. The collection of case types encountered (used to build the type slice)
    // 2. The identifier of the user-given value being processed
    // 3. The detected binding (or the same identifier as the value if none)
    // 4. The pattern to match

    // --- Empty Case --- //
    // This allows usages to contain no cases (e.g., all commented out or macro-generated)
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, } => {{}};

    // --- Else Case --- //
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, else => $action:expr $(,)? } => {
         $action
    };
    {@selector[$($tys:ty,)*] $value:ident, $_binding:ident, $binding:ident @ else => $action:expr $(,)? } => {{
        let $binding: &[$crate::Type] = &[$($crate::Type::of::<$tys>(),)*];
         $action
    }};

    // --- Binding Matcher --- //
    // This rule is used to detect an optional binding (i.e. `<IDENT> @`) for each case.
    // Note that its placement is _below_ the `else` rules.
    // This is to prevent this binding rule from superseding the custom one for the `else` case.
    {@selector[$($tys:ty,)*] $value:ident, $_old_binding:ident, $binding:ident @ $($tt:tt)+} => {
        select_ty!(@selector[$($tys,)*] $value, $binding, $($tt)+)
    };

    // --- Main Cases --- //
    // Note that each main case comes with two rules: a non-terminal and a terminal rule.
    // The non-terminal rule is the one that can be used as an expression since it should be exhaustive.
    // The terminal rule is the one that can be used for non-exhaustive statements.

    // ~~~ Mutable Borrow ~~~ //
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, &mut $ty:ty => $action:expr , $($tt:tt)+} => {
        match $value.as_partial_reflect_mut().try_downcast_mut::<$ty>() {
            Some($binding) => $action,
            None => select_ty!(@selector[$($tys,)* &mut $ty,] $value, $value, $($tt)+)
        }
    };
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, &mut $ty:ty => $action:expr $(,)?} => {
        if let Some($binding) = $value.as_partial_reflect_mut().try_downcast_mut::<$ty>() {
            $action
        }
    };

    // ~~~ Immutable Borrow ~~~ //
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, &$ty:ty => $action:expr , $($tt:tt)+} => {
        match $value.as_partial_reflect().try_downcast_ref::<$ty>() {
            Some($binding) => $action,
            None => select_ty!(@selector[$($tys,)* &mut $ty,] $value, $value, $($tt)+)
        }
    };
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, &$ty:ty => $action:expr $(,)?} => {
        if let Some($binding) = $value.as_partial_reflect().try_downcast_ref::<$ty>() {
            $action
        }
    };

    // ~~~ Owned ~~~ //
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, $ty:ty => $action:expr , $($tt:tt)+} => {
        match $value.into_partial_reflect().try_take::<$ty>() {
            Ok(mut $binding) => $action,
            Err(mut $value) => select_ty!(@selector[$($tys,)* $ty,] $value, $value, $($tt)+),
        }
    };
    {@selector[$($tys:ty,)*] $value:ident, $binding:ident, $ty:ty => $action:expr $(,)?} => {
        if let Ok($binding) = $value.into_partial_reflect().try_take::<$ty>() {
            $action
        }
    };
}

pub use select_ty;

#[cfg(test)]
mod tests {
    #![allow(
        clippy::allow_attributes,
        unused_imports,
        unused_parens,
        unused_mut,
        unused_variables,
        reason = "the warnings generated by these macros should only be visible to `bevy_reflect`"
    )]

    use super::*;
    use crate::{PartialReflect, Type};
    use alloc::boxed::Box;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;
    use alloc::{format, vec};

    #[test]
    fn should_allow_empty() {
        fn empty(_value: Box<dyn PartialReflect>) {
            select_ty! {_value}
            select_ty! {_value,}
        }

        empty(Box::new(42));
    }

    #[test]
    fn should_downcast_ref() {
        fn to_string(value: &dyn PartialReflect) -> String {
            select_ty! {value,
                &String => value.clone(),
                &i32 => value.to_string(),
                &f32 => format!("{:.2}", value),
                else => "unknown".to_string()
            }
        }

        assert_eq!(to_string(&String::from("hello")), "hello");
        assert_eq!(to_string(&42_i32), "42");
        assert_eq!(to_string(&1.2345_f32), "1.23");
        assert_eq!(to_string(&true), "unknown");
    }

    #[test]
    fn should_downcast_mut() {
        fn push_value(container: &mut dyn PartialReflect, value: i32) -> bool {
            select_ty! {container,
                &mut Vec<i32> => container.push(value),
                &mut Vec<u32> => container.push(value as u32),
                else => return false
            }

            true
        }

        let mut list: Vec<i32> = vec![1, 2];
        assert!(push_value(&mut list, 3));
        assert_eq!(list, vec![1, 2, 3]);

        let mut list: Vec<u32> = vec![1, 2];
        assert!(push_value(&mut list, 3));
        assert_eq!(list, vec![1, 2, 3]);

        let mut list: Vec<String> = vec![String::from("hello")];
        assert!(!push_value(&mut list, 3));
    }

    #[test]
    fn should_downcast_owned() {
        fn into_string(value: Box<dyn PartialReflect>) -> Option<String> {
            select_ty! {value,
                String => Some(value),
                i32 => Some(value.to_string()),
                else => None
            }
        }

        let value = Box::new("hello".to_string());
        let result = into_string(value);
        assert_eq!(result, Some("hello".to_string()));

        let value = Box::new(42);
        let result = into_string(value);
        assert_eq!(result, Some("42".to_string()));

        let value = Box::new(true);
        let result = into_string(value);
        assert_eq!(result, None);
    }

    #[test]
    fn should_allow_mixed_borrows() {
        fn process(value: Box<dyn PartialReflect>) {
            select_ty! {value,
                Option<f32> => {
                    let value = value.unwrap();
                    assert_eq!(value, 1.0);
                    return;
                },
                &Option<i32> => {
                    let value = value.as_ref().unwrap();
                    assert_eq!(*value, 42);
                    return;
                },
                &mut Option<String> => {
                    let value = value.as_mut().unwrap();
                    value.push_str(" world");
                    assert_eq!(*value, "hello world");
                    return;
                },
            }

            panic!("test should not reach here");
        }

        process(Box::new(Some(String::from("hello"))));
        process(Box::new(Some(42_i32)));
        process(Box::new(Some(1.0_f32)));
    }

    #[test]
    fn should_allow_custom_bindings() {
        fn process(mut value: Box<dyn PartialReflect>) {
            select_ty! {value,
                foo @ &mut i32 => {
                    *foo *= 2;
                    assert_eq!(*foo, 246);
                    return;
                },
                bar @ &u32 => {
                    assert_eq!(*bar, 42);
                    return;
                },
                baz @ bool => {
                    assert!(baz);
                    return;
                },
            }

            panic!("test should not reach here");
        }

        process(Box::new(123_i32));
        process(Box::new(42_u32));
        process(Box::new(true));
    }

    #[test]
    fn should_allow_else_with_binding() {
        let _value = Box::new(123);

        select_ty! {_value,
            f32 => {
                assert_eq!(_value, 123.0);
            },
            f64 => {
                assert_eq!(_value, 123.0);
            },
            types @ else => {
                assert_eq!(types.len(), 2);
                assert_eq!(types[0], Type::of::<f32>());
                assert_eq!(types[1], Type::of::<f64>());
            },
        }
    }

    #[test]
    fn should_handle_slice_types() {
        let _value = Box::new("hello world");

        select_ty! {_value,
            (&str) => {},
            (&[i32]) => {
                panic!("unexpected type");
            },
            (&mut [u32]) => {
                panic!("unexpected type");
            },
            else => panic!("unexpected type"),
        }
    }
}
