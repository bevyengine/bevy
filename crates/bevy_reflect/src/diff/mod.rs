//! Tools for diffing two [`Reflect`] objects.
//!
//! The core of diffing revolves around the [`Diff`] and [`DiffType`] enums.
//! With these enums, diffs can be generated recursively for all reflect types.
//!
//! When diffing, the two objects are often referred to as "old" and "new".
//! This use of this particular language is purely for clarity's sake and does not necessarily
//! indicate that the "old" value is to be replaced by the "new" one.
//! These terms better indicate the directionality of the diffing operations,
//! which asks _"how can we transform `old` into `new`?"_
//! Other terms include "src" and "dst" as well as "a" and "b".
//!
//! To compute the diff between two objects, use the [`Reflect::diff`] method.
//! This will return the [`Diff`] or an error if diffing failed.
//!
//! With this, we can determine whether a value was [modified], [replaced], or had [no change].
//! When a value is [modified], it contains data related to the modification,
//! which may recursively contain more [`Diff`] objects.
//!
//! # Applying Diffs
//!
//! Diffs store the changes between two values, but they also allow for these changes to be applied to a value.
//!
//! To apply a diff, you can use either the [`Reflect::apply_diff`] method or the [`Diff::apply`] method.
//!
//! Note that diff's hold on to references to both the "old" and "new" values.
//! This means you won't be able to apply a diff to the "old" value unless you clone the diff
//! with [`Diff::clone_diff`] first.
//!
//! ```
//! # use bevy_reflect::Reflect;
//! let old = (1, 2, 3);
//! let new = (0, 2, 4);
//!
//! let diff = old.diff(&new).unwrap();
//!
//! let mut value = (1, 2, 3);
//! diff.apply(&mut value).unwrap();
//! assert_eq!(value, new);
//! ```
//!
//! # Lists & Maps
//!
//! It's important to note that both [List](crate::List) and [Map](crate::Map) types work a bit differently
//! than the other types.
//! This is due to the fact that their size fields are not known at compile time.
//! For example, a list can grow and shrink dynamically, and a map can add or remove entries just as easily.
//!
//! This means there has to be a better approach to representing their diffs that take such factors
//! into account.
//!
//! ## Lists
//!
//! [Lists](crate::List) are diffed using the [Myers Diffing Algorithm].
//! Instead of diffing elements individually in sequence, we try to find the minimum number of edits
//! to transform the "old" list into the "new" one.
//!
//! The available edits are [`ElementDiff::Inserted`] and [`ElementDiff::Deleted`].
//! When calling [`ListDiff::iter_changes`], we iterate over a collection of these edits.
//! Each edit is given an index to determine where the transformation should take place in the "old" list.
//! [`ElementDiff::Deleted`] edits are given the index of the element to delete,
//! while [`ElementDiff::Inserted`] edits are given both the index of the element they should appear _before_
//! as well as the actual data to insert.
//!
//! Note: Multiple inserts may share the same index.
//! This is because, as far as each insertion is concerned, they all come before the element in the
//! "old" list at that index.
//!
//! ```
//! # use bevy_reflect::{Reflect, diff::{Diff, DiffType, ElementDiff}};
//! let old = vec![8, -1, 5];
//! let new = vec![9, 8, 7, 6, 5];
//!
//! let diff = old.diff(&new).unwrap();
//!
//! if let Diff::Modified(DiffType::List(list_diff)) = diff {
//!   let mut changes = list_diff.iter_changes();
//!
//!   assert!(matches!(changes.next(), Some(ElementDiff::Inserted(0, _))));
//!   assert!(matches!(changes.next(), Some(ElementDiff::Deleted(1))));
//!   assert!(matches!(changes.next(), Some(ElementDiff::Inserted(2, _))));
//!   assert!(matches!(changes.next(), Some(ElementDiff::Inserted(2, _))));
//!   assert!(matches!(changes.next(), None));
//! }
//! ```
//!
//! ## Maps
//!
//! [Maps](crate::Map) also include edits for [insertion](EntryDiff::Inserted) and [deletion](EntryDiff::Deleted),
//! but contain a third option: [`EntryDiff::Modified`].
//! Unlike lists, these edits are unordered and do not make use of the [Myers Diffing Algorithm].
//! Instead, the [`EntryDiff::Inserted`] and [`EntryDiff::Deleted`] edits simply indicate whether an entry with a given
//! key was inserted or deleted,
//! while the [`EntryDiff::Modified`] edit indicates that the _value_ of an entry was edited.
//!
//! ```
//! # use bevy_reflect::{Reflect, diff::{Diff, DiffType, EntryDiff}};
//! # use bevy_utils::HashMap;
//! let old = HashMap::from([(1, 111), (2, 222), (3, 333)]);
//! let new = HashMap::from([(2, 999), (3, 333), (4, 444)]);
//!
//! let diff = old.diff(&new).unwrap();
//!
//! if let Diff::Modified(DiffType::Map(map_diff)) = diff {
//!   for change in map_diff.iter_changes() {
//!     match change {
//!       EntryDiff::Deleted(key) => {
//!         assert!(key.reflect_partial_eq(&1).unwrap(), "expected key 1 to be deleted");
//!       }
//!       EntryDiff::Inserted(key, value) => {
//!         assert!(
//!           key.reflect_partial_eq(&4).unwrap() && value.reflect_partial_eq(&444).unwrap(),
//!           "expected key 4 to be inserted with value 444"
//!         );
//!       }
//!       EntryDiff::Modified(key, value_diff) => {
//!         assert!(
//!           key.reflect_partial_eq(&2).unwrap() && matches!(value_diff, Diff::Modified(..)),
//!           "expected key 2 to be modified"
//!         );
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! [`Reflect`]: crate::Reflect
//! [`Reflect::diff`]: crate::Reflect::diff
//! [`Diff`]: crate::diff::Diff
//! [`DiffType`]: crate::diff::DiffType
//! [modified]: Diff::Modified
//! [replaced]: Diff::Replaced
//! [no change]: Diff::NoChange
//! [`Reflect::apply_diff`]: crate::Reflect::apply_diff
//! [Myers Diffing Algorithm]: http://www.xmailserver.org/diff2.pdf

mod array_diff;
#[allow(clippy::module_inception)]
mod diff;
mod enum_diff;
mod error;
mod list_diff;
mod map_diff;
mod struct_diff;
mod tuple_diff;
mod tuple_struct_diff;
mod value_diff;

pub use array_diff::*;
pub use diff::*;
pub use enum_diff::*;
pub use error::*;
pub use list_diff::*;
pub use map_diff::*;
pub use struct_diff::*;
pub use tuple_diff::*;
pub use tuple_struct_diff::*;
pub use value_diff::*;

#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::diff::{
        Diff, DiffApplyError, DiffResult, DiffType, ElementDiff, EntryDiff, EnumDiff,
    };
    use crate::Reflect;
    use bevy_utils::HashMap;

    /// Generates assertions for applying a diff to a value.
    ///
    /// Note that this macro will modify `$old`.
    ///
    /// # Cases
    ///
    /// * [`Diff::NoChange`] - Asserts that applying the diff should result in no change.
    /// * [`Diff::Replaced`] - Asserts that applying the diff should result in a type mismatch error.
    /// * [`Diff::Modified`] - Asserts that applying the diff should result in a conversion from `$old` to `$new`.
    macro_rules! assert_apply_diff {
        ($old: expr, $new: expr, $diff: expr) => {{
            let mut old = $old;
            let new = $new;
            let diff = $diff;

            match &diff {
                Diff::NoChange => {
                    assert!(
                        old.reflect_partial_eq(new.as_reflect()).unwrap_or_default(),
                        "old should be the same as new when diff is `Diff::NoChange`"
                    );
                    diff.apply(old.as_reflect_mut()).unwrap();
                    assert!(
                        old.reflect_partial_eq(new.as_reflect()).unwrap_or_default(),
                        "applying `Diff::NoChange` should result in no change"
                    );
                }
                Diff::Replaced(..) => {
                    let result = diff.apply(old.as_reflect_mut());
                    assert_eq!(
                        result,
                        Err(DiffApplyError::TypeMismatch),
                        "applying `Diff::Replaced` should result in a type mismatch error"
                    );
                }
                Diff::Modified(..) => {
                    diff.apply(old.as_reflect_mut()).unwrap();
                    assert!(
                        old.reflect_partial_eq(new.as_reflect()).unwrap_or_default(),
                        "applying `Diff::Modified` should result in a modified value"
                    );
                }
            };
        }};
    }

    /// Runs a series of tests for diffing two values using the given callback.
    ///
    /// This function will generate four tests:
    /// * diff(concrete, concrete)
    /// * diff(concrete, dynamic)
    /// * diff(dynamic, concrete)
    /// * diff(dynamic, dynamic)
    ///
    /// The callback will be given the results of each diff operation.
    ///
    /// This is useful for testing the consistency of diffing both concrete and dynamic values.
    fn test_diff<T1: Reflect + Clone, T2: Reflect + Clone>(
        old: T1,
        new: T2,
        callback: impl Fn(Box<dyn Reflect>, &dyn Reflect, DiffResult),
    ) {
        // 1. diff(concrete, concrete)
        let diff = old.diff(new.as_reflect());
        callback(Box::new(old.clone()), new.as_reflect(), diff);

        // 2. diff(concrete, dynamic)
        let new_dynamic = new.clone_value();
        let diff = old.diff(new_dynamic.as_reflect());
        callback(Box::new(old.clone()), new_dynamic.as_reflect(), diff);

        // 3. diff(dynamic, concrete)
        let old_dynamic = old.clone_value();
        let diff = old_dynamic.diff(new.as_reflect());
        callback(old_dynamic.clone_value(), new.as_reflect(), diff);

        // 4. diff(dynamic, dynamic)
        let old_dynamic = old.clone_value();
        let new_dynamic = new.clone_value();
        let diff = old_dynamic.diff(new_dynamic.as_reflect());
        callback(old_dynamic.clone_value(), new_dynamic.as_reflect(), diff);
    }

    #[test]
    fn should_diff_value() {
        let old = 123_i32;
        let new = 123_i32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange));
        assert_apply_diff!(old, new, diff);

        let old = 123_i32;
        let new = 321_i32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Modified(..)));
        assert_apply_diff!(old, new, diff);

        let old = 123_i32;
        let new = 123_u32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));
        assert_apply_diff!(old, new, diff);
    }

    #[test]
    fn should_diff_tuple() {
        test_diff((1, 2, 3), (1, 2, 3), |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::NoChange)));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff((1, 2, 3), (1, 2, 3, 4), |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::Replaced(..))));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff((1, 2, 3), (1, 0, 3), |old, new, diff| {
            if let Ok(Diff::Modified(modified)) = &diff {
                if let DiffType::Tuple(tuple_diff) = modified {
                    let mut fields = tuple_diff.field_iter();

                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(matches!(
                        fields.next(),
                        Some(Diff::Modified(DiffType::Value(..)))
                    ));
                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(fields.next().is_none());
                } else {
                    panic!("expected `DiffType::Tuple`");
                }
            } else {
                panic!("expected `Diff::Modified`");
            }
            assert_apply_diff!(old, new, diff.unwrap());
        });
    }

    #[test]
    fn should_diff_array() {
        test_diff([1, 2, 3], [1, 2, 3], |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::NoChange)));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff([1, 2, 3], [1, 2, 3, 4], |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::Replaced(..))));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff([1, 2, 3], [1, 0, 3], |old, new, diff| {
            if let Ok(Diff::Modified(modified)) = &diff {
                if let DiffType::Array(array_diff) = modified {
                    let mut fields = array_diff.iter();

                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(matches!(
                        fields.next(),
                        Some(Diff::Modified(DiffType::Value(..)))
                    ));
                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(fields.next().is_none());
                } else {
                    panic!("expected `DiffType::Array`");
                }
            } else {
                panic!("expected `Diff::Modified`");
            }
            assert_apply_diff!(old, new, diff.unwrap());
        });
    }

    #[test]
    fn should_diff_list() {
        test_diff(vec![1, 2, 3], vec![1, 2, 3], |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::NoChange)));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff(
            vec![1, 2, 3] as Vec<i32>,
            vec![1u32, 2, 3] as Vec<u32>,
            |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::Replaced(..))));
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(vec![1, 2, 3], vec![9, 1, 2, 3], |old, new, diff| {
            if let Ok(Diff::Modified(modified)) = &diff {
                if let DiffType::List(list_diff) = modified {
                    let mut changes = list_diff.iter_changes();

                    assert!(matches!(
                        changes.next(),
                        Some(ElementDiff::Inserted(0, _ /* 9 */))
                    ));
                    assert!(changes.next().is_none());
                } else {
                    panic!("expected `DiffType::List`");
                }
            } else {
                panic!("expected `Diff::Modified`");
            }
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff(
            vec![] as Vec<i32>,
            vec![1, 2, 3] as Vec<i32>,
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::List(list_diff) = modified {
                        let mut changes = list_diff.iter_changes();

                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(0, _ /* 1 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(0, _ /* 2 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(0, _ /* 3 */))
                        ));
                        assert!(changes.next().is_none());
                    } else {
                        panic!("expected `DiffType::List`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            vec![1, 2, 3, 4, 5],
            vec![1, 0, 3, 6, 8, 4, 7],
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::List(list_diff) = modified {
                        let mut changes = list_diff.iter_changes();

                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Deleted(1 /* 2 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(2, _ /* 0 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(3, _ /* 6 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(3, _ /* 8 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Deleted(4 /* 5 */))
                        ));
                        assert!(matches!(
                            changes.next(),
                            Some(ElementDiff::Inserted(5, _ /* 7 */))
                        ));
                        assert!(changes.next().is_none());
                    } else {
                        panic!("expected `DiffType::List`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );
    }

    #[test]
    fn should_diff_map() {
        macro_rules! map {
            ($($key: tt : $value: expr),* $(,)?) => {
                HashMap::from([$((($key, $value))),*])
            };
        }

        test_diff(
            map! {1: 111, 2: 222, 3: 333},
            map! {1: 111, 2: 222, 3: 333},
            |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::NoChange)));
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            map! {1: 111, 2: 222, 3: 333} as HashMap<i32, i32>,
            map! {1: 111u32, 2: 222, 3: 333} as HashMap<i32, u32>,
            |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::Replaced(..))));
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            map! {1: 111, 2: 222, 3: 333},
            map! {1: 111, 3: 333},
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::Map(map_diff) = modified {
                        let mut changes = map_diff.iter_changes();

                        assert!(matches!(
                            changes.next(),
                            Some(EntryDiff::Deleted(_ /* 2 */))
                        ));
                        assert!(changes.next().is_none());
                    } else {
                        panic!("expected `DiffType::Map`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            map! {1: 111, 2: 222, 3: 333},
            map! {1: 111, 2: 222, 3: 333, 4: 444},
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::Map(map_diff) = modified {
                        let mut changes = map_diff.iter_changes();

                        assert!(matches!(
                            changes.next(),
                            Some(EntryDiff::Inserted(_ /* 4 */, _ /* 444 */))
                        ));
                        assert!(changes.next().is_none());
                    } else {
                        panic!("expected `DiffType::Map`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            map! {1: 111, 2: 222, 3: 333},
            map! {1: 111, 2: 999, 3: 333},
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::Map(map_diff) = modified {
                        let mut changes = map_diff.iter_changes();

                        assert!(matches!(
                            changes.next(),
                            Some(EntryDiff::Modified(_ /* 2 */, _ /* 999 */))
                        ));
                        assert!(changes.next().is_none());
                    } else {
                        panic!("expected `DiffType::Map`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );
    }

    #[test]
    fn should_diff_tuple_struct() {
        #[derive(Reflect, Clone)]
        struct Foo(i32, i32, i32);
        #[derive(Reflect, Clone)]
        struct Bar(i32, i32, i32, i32);

        test_diff(Foo(1, 2, 3), Foo(1, 2, 3), |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::NoChange)));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff(Foo(1, 2, 3), Bar(1, 2, 3, 4), |old, new, diff| {
            assert!(matches!(diff, Ok(Diff::Replaced(..))));
            assert_apply_diff!(old, new, diff.unwrap());
        });

        test_diff(Foo(1, 2, 3), Foo(1, 0, 3), |old, new, diff| {
            if let Ok(Diff::Modified(modified)) = &diff {
                if let DiffType::TupleStruct(tuple_struct_diff) = modified {
                    let mut fields = tuple_struct_diff.field_iter();

                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(matches!(
                        fields.next(),
                        Some(Diff::Modified(DiffType::Value(..)))
                    ));
                    assert!(matches!(fields.next(), Some(Diff::NoChange)));
                    assert!(fields.next().is_none());
                } else {
                    panic!("expected `DiffType::TupleStruct`");
                }
            } else {
                panic!("expected `Diff::Modified`");
            }
            assert_apply_diff!(old, new, diff.unwrap());
        });
    }

    #[test]
    fn should_diff_struct() {
        #[derive(Reflect, Clone)]
        struct Foo {
            a: i32,
            b: f32,
        }
        #[derive(Reflect, Clone)]
        struct Bar {
            a: i32,
            b: f32,
            c: usize,
        }

        test_diff(
            Foo { a: 123, b: 1.23 },
            Foo { a: 123, b: 1.23 },
            |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::NoChange)));
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            Foo { a: 123, b: 1.23 },
            Bar {
                a: 123,
                b: 1.23,
                c: 123,
            },
            |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::Replaced(..))));
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );

        test_diff(
            Foo { a: 123, b: 1.23 },
            Foo { a: 123, b: 3.21 },
            |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::Struct(struct_diff) = modified {
                        let mut fields = struct_diff.field_iter();

                        assert!(matches!(fields.next(), Some(("a", Diff::NoChange))));
                        assert!(matches!(
                            fields.next(),
                            Some(("b", Diff::Modified(DiffType::Value(..))))
                        ));
                        assert!(fields.next().is_none());
                    } else {
                        panic!("expected `DiffType::Struct`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            },
        );
    }

    mod enums {
        use super::*;

        #[test]
        fn should_diff_unit_variant() {
            #[derive(Reflect, Clone)]
            enum Foo {
                A,
                B,
            }
            #[derive(Reflect, Clone)]
            enum Bar {
                A,
                B,
            }

            test_diff(Foo::A, Foo::A, |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::NoChange)));
                assert_apply_diff!(old, new, diff.unwrap());
            });

            test_diff(Foo::A, Foo::B, |old, new, diff| {
                assert!(matches!(
                    diff,
                    Ok(Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..))))
                ));
                assert_apply_diff!(old, new, diff.unwrap());
            });

            test_diff(Foo::A, Bar::A, |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::Replaced(..))));
                assert_apply_diff!(old, new, diff.unwrap());
            });
        }

        #[test]
        fn should_diff_tuple_variant() {
            #[derive(Reflect, Clone)]
            enum Foo {
                A(i32, i32, i32),
                B(i32, i32, i32),
            }
            #[derive(Reflect, Clone)]
            enum Bar {
                A(i32, i32, i32),
                B(i32, i32, i32),
            }

            test_diff(Foo::A(1, 2, 3), Foo::A(1, 2, 3), |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::NoChange)));
                assert_apply_diff!(old, new, diff.unwrap());
            });

            test_diff(Foo::A(1, 2, 3), Foo::B(1, 2, 3), |old, new, diff| {
                assert!(matches!(
                    diff,
                    Ok(Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..))))
                ));
                assert_apply_diff!(old, new, diff.unwrap());
            });

            test_diff(Foo::A(1, 2, 3), Bar::A(1, 2, 3), |old, new, diff| {
                assert!(matches!(diff, Ok(Diff::Replaced(..))));
                assert_apply_diff!(old, new, diff.unwrap());
            });

            test_diff(Foo::A(1, 2, 3), Foo::A(1, 0, 3), |old, new, diff| {
                if let Ok(Diff::Modified(modified)) = &diff {
                    if let DiffType::Enum(enum_diff) = modified {
                        if let EnumDiff::Tuple(tuple_diff) = enum_diff {
                            let mut fields = tuple_diff.field_iter();

                            assert!(matches!(fields.next(), Some(Diff::NoChange)));
                            assert!(matches!(
                                fields.next(),
                                Some(Diff::Modified(DiffType::Value(..)))
                            ));
                            assert!(matches!(fields.next(), Some(Diff::NoChange)));
                            assert!(fields.next().is_none());
                        } else {
                            panic!("expected `EnumDiff::Tuple`");
                        }
                    } else {
                        panic!("expected `DiffType::Enum`");
                    }
                } else {
                    panic!("expected `Diff::Modified`");
                }
                assert_apply_diff!(old, new, diff.unwrap());
            });
        }

        #[test]
        fn should_diff_struct_variant() {
            #[derive(Reflect, Clone)]
            enum Foo {
                A { x: f32, y: f32 },
                B { x: f32, y: f32 },
            }
            #[derive(Reflect, Clone)]
            enum Bar {
                A { x: f32, y: f32 },
                B { x: f32, y: f32 },
            }

            test_diff(
                Foo::A { x: 1.23, y: 4.56 },
                Foo::A { x: 1.23, y: 4.56 },
                |old, new, diff| {
                    assert!(matches!(diff, Ok(Diff::NoChange)));
                    assert_apply_diff!(old, new, diff.unwrap());
                },
            );

            test_diff(
                Foo::A { x: 1.23, y: 4.56 },
                Foo::B { x: 1.23, y: 4.56 },
                |old, new, diff| {
                    assert!(matches!(
                        diff,
                        Ok(Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..))))
                    ));
                    assert_apply_diff!(old, new, diff.unwrap());
                },
            );

            test_diff(
                Foo::A { x: 1.23, y: 4.56 },
                Bar::A { x: 1.23, y: 4.56 },
                |old, new, diff| {
                    assert!(matches!(diff, Ok(Diff::Replaced(..))));
                    assert_apply_diff!(old, new, diff.unwrap());
                },
            );

            test_diff(
                Foo::A { x: 1.23, y: 4.56 },
                Foo::A { x: 1.23, y: 7.89 },
                |old, new, diff| {
                    if let Ok(Diff::Modified(modified)) = &diff {
                        if let DiffType::Enum(enum_diff) = modified {
                            if let EnumDiff::Struct(struct_diff) = enum_diff {
                                let mut fields = struct_diff.field_iter();

                                assert!(matches!(fields.next(), Some(("x", Diff::NoChange))));
                                assert!(matches!(
                                    fields.next(),
                                    Some(("y", Diff::Modified(DiffType::Value(..))))
                                ));
                                assert!(fields.next().is_none());
                            } else {
                                panic!("expected `EnumDiff::Struct`");
                            }
                        } else {
                            panic!("expected `DiffType::Enum`");
                        }
                    } else {
                        panic!("expected `Diff::Modified`");
                    }
                    assert_apply_diff!(old, new, diff.unwrap());
                },
            );
        }
    }

    #[test]
    fn diff_should_be_clonable() {
        let old = vec![1, 2, 3];
        let new = vec![0, 2, 4];

        let diff = old.diff(&new).unwrap();
        let cloned = diff.clone_diff();

        let mut value = vec![1, 2, 3];
        value.apply_diff(cloned).unwrap();

        assert_eq!(value, new);
    }
}
