/// A helper macro for downcasting a [`PartialReflect`] value to a concrete type.
///
/// # Syntax
///
/// The syntax of the macro closely resembles a standard `match`, but with some subtle differences.
///
/// ```text
/// select_type! { <input>, <arms> }
///
/// <input> := IDENT                          // the variable you’re matching
///
/// <arms> :=
///     <arm> ( , <arm> )* [ , ]              // zero or more arms with optional trailing comma
///
/// <arm> :=
///     [ <binding> @ ] <pattern> [ where <cond> ] => <expr>
///
/// <arm> :=
///     [ <value_binding> @ ]                 // *optional* binding for the downcasted value
///     <pattern>                             // type pattern to match
///     [ `[` <types_binding> `]` ]           // *optional* type array
///     [ where <cond> ]                      // *optional* guard
///     => <expr>                             // expression or block to run on match
///
/// <value_binding> := IDENT | _      
///
/// <types_binding> := IDENT                                             
///
/// <binding> := IDENT | _                    // rename or ignore the downcast value
///
/// <pattern> :=                              // determines the downcast method
///   | TYPE                                  // -> try_take::<TYPE>()
///   | & TYPE                                // -> try_downcast_ref::<TYPE>()
///   | &mut TYPE                             // -> try_downcast_mut::<TYPE>()
///   | _                                     // -> catch‑all (no downcast)
///
/// <cond> := a boolean expression that acts as a guard for the arm
/// <expr> := expression or block that runs if the downcast succeeds for the given type
/// ```
///
/// The `<input>` must be a type that implements [`PartialReflect`].
/// Owned values should be passed as a `Box<dyn PartialReflect>`.
///
/// Types are matched in the order they are defined.
/// Any `_` cases must be the last case in the list.
///
/// If a custom binding is not provided,
/// the downcasted value will be bound to the same identifier as the input, thus shadowing it.
/// You can use `_` to ignore the downcasted value if you don’t need it,
/// which may be helpful to silence any "unused variable" lints.
///
/// The `where` clause is optional and can be used to add an extra boolean guard.
/// If the guard evaluates to `true`, the expression will be executed if the type matches.
/// Otherwise, matching will continue to the next arm even if the type matches.
///
/// If a `<types_binding>` is defined, this will be bound to a slice of [`Type`]s
/// that were checked up to and including the current arm.
/// This can be useful for debugging or logging purposes.
/// Note that this list may contain duplicates if the same type is checked in multiple arms,
/// such as when using `where` guards.
///
/// # Examples
///
/// ```
/// # use bevy_reflect::macros::select_type;
/// # use bevy_reflect::PartialReflect;
/// #
/// fn stringify(mut value: Box<dyn PartialReflect>) -> String {
///     select_type! { value,
///         // Downcast to an owned type
///         f32 => format!("{:.1}", value),
///         // Downcast to a mutable reference
///         &mut i32 => {
///             *value *= 2;
///             value.to_string()
///         },
///         // Define custom bindings
///         chars @ &Vec<char> => chars.iter().collect(),
///         // Define conditional guards
///         &String where value == "ping" => "pong".to_owned(),
///         &String => value.clone(),
///         // Fallback case with an optional type array
///         _ [types] => {
///             println!("Couldn't match any types: {:?}", types);
///             "<unknown>".to_string()
///         },
///     }
/// }
///
/// assert_eq!(stringify(Box::new(123.0_f32)), "123.0");
/// assert_eq!(stringify(Box::new(123_i32)), "246");
/// assert_eq!(stringify(Box::new(vec!['h', 'e', 'l', 'l', 'o'])), "hello");
/// assert_eq!(stringify(Box::new("ping".to_string())), "pong");
/// assert_eq!(stringify(Box::new("hello".to_string())), "hello");
/// assert_eq!(stringify(Box::new(true)), "<unknown>");
/// ```
///
/// [`PartialReflect`]: crate::PartialReflect
/// [`Type`]: crate::Type
#[macro_export]
macro_rules! select_type {

    // === Entry Point === //

    {$input:ident} => {{}};
    {$input:ident, $($tt:tt)*} => {{
        // We use an import over fully-qualified syntax so users don't have to
        // cast to `dyn PartialReflect` or dereference manually
        use $crate::PartialReflect;

        select_type!(@arm[[], $input] $($tt)*)
    }};

    // === Arm Parsing === //
    // These rules take the following input (in `[]`):
    // 1. The input identifier
    // 2. An optional binding identifier (or `_`)
    //
    // Additionally, most cases are comprised of both a terminal and non-terminal rule.

    // --- Empty Case --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?]} => {{}};

    // --- Custom Bindings --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] $new_binding:tt @ $($tt:tt)+} => {
        select_type!(@arm [[$($tys),*], $input as $new_binding] $($tt)+)
    };

    // --- Fallback Case --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] _ $([$types:ident])? $(where $condition:expr)? => $action:expr, $($tt:tt)+} => {
        select_type!(@else [[$($tys),*], $input $(as $binding)?, [$($types)?], [$($condition)?], $action] $($tt)+)
    };
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] _ $([$types:ident])? $(where $condition:expr)? => $action:expr $(,)?} => {
        select_type!(@else [[$($tys),*], $input $(as $binding)?, [$($types)?], [$($condition)?], $action])
    };

    // --- Mutable Downcast Case --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] &mut $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr, $($tt:tt)+} => {
        select_type!(@if [[$($tys,)* &mut $ty], $input $(as $binding)?, mut, $ty, [$($types)?], [$($condition)?], $action] $($tt)+)
    };
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] &mut $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr $(,)?} => {
        select_type!(@if [[$($tys,)* &mut $ty], $input $(as $binding)?, mut, $ty, [$($types)?], [$($condition)?], $action])
    };

    // --- Immutable Downcast Case --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] & $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr, $($tt:tt)+} => {
        select_type!(@if [[$($tys,)* &$ty], $input $(as $binding)?, ref, $ty, [$($types)?], [$($condition)?], $action] $($tt)+)
    };
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] & $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr $(,)?} => {
        select_type!(@if [[$($tys,)* &$ty], $input $(as $binding)?, ref, $ty, [$($types)?], [$($condition)?], $action])
    };

    // --- Owned Downcast Case --- //
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr, $($tt:tt)+} => {
        select_type!(@if [[$($tys,)* $ty], $input $(as $binding)?, box, $ty, [$($types)?], [$($condition)?], $action] $($tt)+)
    };
    {@arm [[$($tys:ty),*], $input:ident $(as $binding:tt)?] $ty:ty $([$types:ident])? $(where $condition:expr)? => $action:expr $(,)?} => {
        select_type!(@if [[$($tys,)* $ty], $input $(as $binding)?, box, $ty, [$($types)?], [$($condition)?], $action])
    };

    // === Type Matching === //
    // These rules take the following input (in `[]`):
    // 1. The input identifier
    // 2. An optional binding identifier (or `_`)
    // 3. The kind of downcast (e.g., `mut`, `ref`, or `box`)
    // 4. The type to downcast to
    // 5. An optional condition (wrapped in `[]` for disambiguation)
    // 6. The action to take if the downcast succeeds

    // This rule handles the owned downcast case
    {@if [[$($tys:ty),*], $input:ident $(as $binding:tt)?, box, $ty:ty, [$($types:ident)?], [$($condition:expr)?], $action:expr] $($rest:tt)*} => {
        #[allow(unused_parens, reason = "may be used for disambiguation")]
        match select_type!(@downcast box, $ty, $input) {
            Ok(select_type!(@bind [mut] $input $(as $binding)?)) $(if $condition)? => {
                select_type!(@collect [$($tys),*] $(as $types)?);
                $action
            },
            $input => {
                // We have to rebind `$value` here so that we can unconditionally ignore it
                // due to the fact that `unused_variables` seems to be the only lint that
                // is visible outside the macro when used within other crates.
                #[allow(
                    unused_variables,
                    reason = "unfortunately this variable cannot receive a custom binding to let it be ignored otherwise"
                )]
                let mut $input = match $input {
                    Ok($input) => $crate::__macro_exports::alloc_utils::Box::new($input) as $crate::__macro_exports::alloc_utils::Box<dyn $crate::PartialReflect>,
                    Err($input) => $input
                };

                select_type!(@arm [[$($tys),*], $input] $($rest)*)
            }
        }
    };
    // This rule handles the mutable and immutable downcast cases
    {@if [[$($tys:ty),*], $input:ident $(as $binding:tt)?, $kind:tt, $ty:ty, [$($types:ident)?], [$($condition:expr)?], $action:expr] $($rest:tt)*} => {
        #[allow(unused_parens, reason = "may be used for disambiguation")]
        match select_type!(@downcast $kind, $ty, $input) {
            Some(select_type!(@bind [] $input $(as $binding)?)) $(if $condition)? => {
                select_type!(@collect [$($tys),*] $(as $types)?);
                $action
            },
            _ => {
                select_type!(@arm [[$($tys),*], $input] $($rest)*)
            }
        }
    };

    // This rule handles the fallback case where a condition has been provided
    {@else [[$($tys:ty),*], $input:ident $(as $binding:tt)?, [$($types:ident)?], [$condition:expr], $action:expr] $($rest:tt)*} => {{
        select_type!(@collect [$($tys),*] $(as $types)?);
        let select_type!(@bind [mut] _ $(as $binding)?) = $input;

        if $condition {
            $action
        } else {
            select_type!(@arm [[$($tys),*], $input] $($rest)*)
        }
    }};
    // This rule handles the fallback case where no condition has been provided
    {@else [[$($tys:ty),*], $input:ident $(as $binding:tt)?, [$($types:ident)?], [], $action:expr] $($rest:tt)*} => {{
        select_type!(@collect [$($tys),*] $(as $types)?);
        let select_type!(@bind [mut] _ $(as $binding)?) = $input;

        $action
    }};

    // === Helpers === //

    // --- Downcasting --- //
    // Helpers for downcasting `$input` to `$ty`
    // based on the given keyword (`mut`, `ref`, or `box`).

    {@downcast mut, $ty:ty, $input:ident} => {
        $input.as_partial_reflect_mut().try_downcast_mut::<$ty>()
    };
    {@downcast ref, $ty:ty, $input:ident} => {
        $input.as_partial_reflect().try_downcast_ref::<$ty>()
    };
    {@downcast box, $ty:ty, $input:ident} => {
        // We eagerly box here so that we can support non-boxed values.
        $crate::__macro_exports::alloc_utils::Box::new($input).into_partial_reflect().try_take::<$ty>()
    };

    // --- Binding --- //
    // Helpers for creating a binding for the downcasted value.
    // This ensures that we only add `mut` when necessary,
    // and that `_` is handled appropriately.

    {@bind [$($mut:tt)?] _} => {
        _
    };
    {@bind [$($mut:tt)?] $input:ident} => {
        $($mut)? $input
    };
    {@bind [$($mut:tt)?] $input:tt as _} => {
        _
    };
    {@bind [$($mut:tt)?] $input:tt as $binding:ident} => {
        $($mut)? $binding
    };

    // --- Collect Types --- //
    // Helpers for collecting the types into an array of `Type`.

    {@collect [$($ty:ty),*]} => {};
    {@collect [$($tys:ty),*] as $types:ident} => {
        let $types: &[$crate::Type] = &[$($crate::Type::of::<$tys>(),)*];
    };
}

pub use select_type;

#[cfg(test)]
mod tests {
    #![allow(
        clippy::allow_attributes,
        unused_imports,
        unused_parens,
        unused_mut,
        reason = "the warnings generated by these macros should only be visible to `bevy_reflect`"
    )]

    use super::*;
    use crate::{PartialReflect, Type};
    use alloc::boxed::Box;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;
    use alloc::{format, vec};
    use core::fmt::Debug;
    use core::ops::MulAssign;

    #[test]
    fn should_allow_empty() {
        fn empty(_value: Box<dyn PartialReflect>) {
            let _: () = select_type! {_value};
            let _: () = select_type! {_value,};
        }

        empty(Box::new(42));
    }

    #[test]
    fn should_downcast_ref() {
        fn to_string(value: &dyn PartialReflect) -> String {
            select_type! {value,
                &String => value.clone(),
                &i32 => value.to_string(),
                &f32 => format!("{:.2}", value),
                _ => "unknown".to_string()
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
            select_type! {container,
                &mut Vec<i32> => container.push(value),
                &mut Vec<u32> => container.push(value as u32),
                _ => return false
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
            select_type! {value,
                String => Some(value),
                i32 => Some(value.to_string()),
                _ => None
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
    fn should_retrieve_owned() {
        let original_value = String::from("hello");
        let cloned_value = original_value.clone();

        let value = select_type! {cloned_value,
            _ @ Option<String> => panic!("unexpected type"),
            _ => cloned_value
        };

        assert_eq!(value.try_take::<String>().unwrap(), *original_value);
    }

    #[test]
    fn should_allow_mixed_borrows() {
        fn process(value: Box<dyn PartialReflect>) {
            select_type! {value,
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
            select_type! {value,
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
    fn should_handle_slice_types() {
        let _value = "hello world";

        select_type! {_value,
            (&str) => {},
            _ => panic!("unexpected type"),
        }
    }

    #[test]
    fn should_capture_types() {
        fn test(mut value: Box<dyn PartialReflect>) {
            select_type! {value,
                _ @ &mut u8 [types] => {
                    assert_eq!(types.len(), 1);
                    assert_eq!(types[0], Type::of::<&mut u8>());
                },
                _ @ &u16 [types] => {
                    assert_eq!(types.len(), 2);
                    assert_eq!(types[0], Type::of::<&mut u8>());
                    assert_eq!(types[1], Type::of::<&u16>());
                },
                u32 [types] where value > 10  => {
                    assert_eq!(types.len(), 3);
                    assert_eq!(types[0], Type::of::<&mut u8>());
                    assert_eq!(types[1], Type::of::<&u16>());
                    assert_eq!(types[2], Type::of::<u32>());
                },
                _ @ u32 [types] => {
                    assert_eq!(types.len(), 4);
                    assert_eq!(types[0], Type::of::<&mut u8>());
                    assert_eq!(types[1], Type::of::<&u16>());
                    assert_eq!(types[2], Type::of::<u32>());
                    assert_eq!(types[3], Type::of::<u32>());
                },
                _ [types] => {
                    assert_eq!(types.len(), 4);
                    assert_eq!(types[0], Type::of::<&mut u8>());
                    assert_eq!(types[1], Type::of::<&u16>());
                    assert_eq!(types[2], Type::of::<u32>());
                    assert_eq!(types[3], Type::of::<u32>());
                },
            }
        }

        test(Box::new(0_u8));
        test(Box::new(0_u16));
        test(Box::new(123_u32));
        test(Box::new(0_u32));
        test(Box::new(0_u64));
    }

    #[test]
    fn should_downcast_from_generic() {
        fn immutable<T: PartialReflect>(value: &T) {
            select_type! {value,
                &i32 => {
                    assert_eq!(*value, 1);
                },
                _ => panic!("unexpected type"),
            }
        }

        fn mutable<T: PartialReflect>(value: &mut T) {
            select_type! {value,
                &mut i32 => {
                    *value = 2;
                },
                _ => panic!("unexpected type"),
            }
        }

        fn owned<T: PartialReflect>(value: T) {
            select_type! {value,
                i32 => {
                    assert_eq!(value, 2);
                },
                _ => panic!("unexpected type"),
            }
        }

        let mut value = 1_i32;
        immutable(&value);
        mutable(&mut value);
        owned(value);
    }

    #[test]
    fn should_downcast_to_generic() {
        fn immutable<T: PartialReflect, U: PartialReflect + Debug + PartialEq<i32>>(value: &T) {
            select_type! {value,
                &U => {
                    assert_eq!(*value, 1);
                },
                _ => panic!("unexpected type"),
            }
        }

        fn mutable<T: PartialReflect, U: PartialReflect + MulAssign<i32>>(value: &mut T) {
            select_type! {value,
                &mut U => {
                    *value *= 2;
                },
                _ => panic!("unexpected type"),
            }
        }

        fn owned<T: PartialReflect, U: PartialReflect + Debug + PartialEq<i32>>(value: T) {
            select_type! {value,
                U => {
                    assert_eq!(value, 2);
                },
                _ => panic!("unexpected type"),
            }
        }

        let mut value = 1_i32;
        immutable::<_, i32>(&value);
        mutable::<_, i32>(&mut value);
        owned::<_, i32>(value);
    }
}
