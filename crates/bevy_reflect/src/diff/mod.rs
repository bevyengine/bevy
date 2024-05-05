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
    use crate::diff::{Diff, DiffType, ElementDiff, EntryDiff, EnumDiff};
    use crate::Reflect;
    use bevy_utils::HashMap;

    #[test]
    fn should_diff_value() {
        let old = 123_i32;
        let new = 123_i32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old = 123_i32;
        let new = 321_i32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Modified(..)));

        let old = 123_i32;
        let new = 123_u32;

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));
    }

    #[test]
    fn should_diff_tuple() {
        let old = (1, 2, 3);
        let new = (1, 2, 3);

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old = (1, 2, 3);
        let new = (1, 2, 3, 4);

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));

        let old = (1, 2, 3);
        let new = (1, 0, 3);

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
            if let DiffType::Tuple(tuple_diff) = modified {
                let mut fields = tuple_diff.field_iter();

                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(matches!(
                    fields.next(),
                    Some(Diff::Modified(DiffType::Value(..)))
                ));
                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(fields.next().is_none());
            } else {
                panic!("expected `DiffType::Tuple`");
            }
        } else {
            panic!("expected `Diff::Modified`");
        }
    }

    #[test]
    fn should_diff_array() {
        let old = [1, 2, 3];
        let new = [1, 2, 3];

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old = [1, 2, 3];
        let new = [1, 2, 3, 4];

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));

        let old = [1, 2, 3];
        let new = [1, 0, 3];

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
            if let DiffType::Array(array_diff) = modified {
                let mut fields = array_diff.iter();

                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(matches!(
                    fields.next(),
                    Some(Diff::Modified(DiffType::Value(..)))
                ));
                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(fields.next().is_none());
            } else {
                panic!("expected `DiffType::Array`");
            }
        } else {
            panic!("expected `Diff::Modified`");
        }
    }

    #[test]
    fn should_diff_list() {
        let old = vec![1, 2, 3];
        let new = vec![1, 2, 3];

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old: Vec<i32> = vec![1, 2, 3];
        let new: Vec<u32> = vec![1, 2, 3];

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(_)));

        let old = vec![1, 2, 3];
        let new = vec![9, 1, 2, 3];

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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

        let old: Vec<i32> = vec![];
        let new: Vec<i32> = vec![1, 2, 3];

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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

        let old = vec![1, 2, 3, 4, 5];
        let new = vec![1, 0, 3, 6, 8, 4, 7];

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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
    }

    #[test]
    fn should_diff_map() {
        macro_rules! map {
            ($($key: tt : $value: expr),* $(,)?) => {
                HashMap::from([$((($key, $value))),*])
            };
        }

        let old = map! {1: 111, 2: 222, 3: 333};
        let new = map! {3: 333, 1: 111, 2: 222};

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old: HashMap<i32, i32> = map! {1: 111, 2: 222, 3: 333};
        let new: HashMap<i32, u32> = map! {1: 111, 2: 222, 3: 333};

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(_)));

        let old = map! {1: 111, 2: 222, 3: 333};
        let new = map! {1: 111, 3: 333};

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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

        let old = map! {1: 111, 2: 222, 3: 333};
        let new = map! {1: 111, 2: 222, 3: 333, 4: 444};

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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

        let old = map! {1: 111, 2: 222, 3: 333};
        let new = map! {1: 111, 2: 999, 3: 333};

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
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
    }

    #[test]
    fn should_diff_tuple_struct() {
        #[derive(Reflect)]
        struct Foo(i32, i32, i32);
        #[derive(Reflect)]
        struct Bar(i32, i32, i32, i32);

        let old = Foo(1, 2, 3);
        let new = Foo(1, 2, 3);

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old = Foo(1, 2, 3);
        let new = Bar(1, 2, 3, 4);

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));

        let old = Foo(1, 2, 3);
        let new = Foo(1, 0, 3);

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
            if let DiffType::TupleStruct(tuple_struct_diff) = modified {
                let mut fields = tuple_struct_diff.field_iter();

                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(matches!(
                    fields.next(),
                    Some(Diff::Modified(DiffType::Value(..)))
                ));
                assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                assert!(fields.next().is_none());
            } else {
                panic!("expected `DiffType::TupleStruct`");
            }
        } else {
            panic!("expected `Diff::Modified`");
        }
    }

    #[test]
    fn should_diff_struct() {
        #[derive(Reflect)]
        struct Foo {
            a: i32,
            b: f32,
        }
        #[derive(Reflect)]
        struct Bar {
            a: i32,
            b: f32,
            c: usize,
        }

        let old = Foo { a: 123, b: 1.23 };
        let new = Foo { a: 123, b: 1.23 };

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::NoChange(_)));

        let old = Foo { a: 123, b: 1.23 };
        let new = Bar {
            a: 123,
            b: 1.23,
            c: 123,
        };

        let diff = old.diff(&new).unwrap();
        assert!(matches!(diff, Diff::Replaced(..)));

        let old = Foo { a: 123, b: 1.23 };
        let new = Foo { a: 123, b: 3.21 };

        let diff = old.diff(&new).unwrap();
        if let Diff::Modified(modified) = diff {
            if let DiffType::Struct(struct_diff) = modified {
                let mut fields = struct_diff.field_iter();

                assert!(matches!(fields.next(), Some(("a", Diff::NoChange(_)))));
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
    }
    mod enums {
        use super::*;

        #[test]
        fn should_diff_unit_variant() {
            #[derive(Reflect)]
            enum Foo {
                A,
                B,
            }
            #[derive(Reflect)]
            enum Bar {
                A,
                B,
            }

            let old = Foo::A;
            let new = Foo::A;

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::NoChange(_)));

            let old = Foo::A;
            let new = Foo::B;

            let diff = old.diff(&new).unwrap();
            assert!(matches!(
                diff,
                Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..)))
            ));

            let old = Foo::A;
            let new = Bar::A;

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::Replaced(..)));
        }

        #[test]
        fn should_diff_tuple_variant() {
            #[derive(Reflect)]
            enum Foo {
                A(i32, i32, i32),
                B(i32, i32, i32),
            }
            #[derive(Reflect)]
            enum Bar {
                A(i32, i32, i32),
                B(i32, i32, i32),
            }

            let old = Foo::A(1, 2, 3);
            let new = Foo::A(1, 2, 3);

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::NoChange(_)));

            let old = Foo::A(1, 2, 3);
            let new = Foo::B(1, 2, 3);

            let diff = old.diff(&new).unwrap();
            assert!(matches!(
                diff,
                Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..)))
            ));

            let old = Foo::A(1, 2, 3);
            let new = Bar::A(1, 2, 3);

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::Replaced(..)));

            let old = Foo::A(1, 2, 3);
            let new = Foo::A(1, 0, 3);

            let diff = old.diff(&new).unwrap();
            if let Diff::Modified(modified) = diff {
                if let DiffType::Enum(enum_diff) = modified {
                    if let EnumDiff::Tuple(tuple_diff) = enum_diff {
                        let mut fields = tuple_diff.field_iter();

                        assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
                        assert!(matches!(
                            fields.next(),
                            Some(Diff::Modified(DiffType::Value(..)))
                        ));
                        assert!(matches!(fields.next(), Some(Diff::NoChange(_))));
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
        }

        #[test]
        fn should_diff_struct_variant() {
            #[derive(Reflect)]
            enum Foo {
                A { x: f32, y: f32 },
                B { x: f32, y: f32 },
            }
            #[derive(Reflect)]
            enum Bar {
                A { x: f32, y: f32 },
                B { x: f32, y: f32 },
            }

            let old = Foo::A { x: 1.23, y: 4.56 };
            let new = Foo::A { x: 1.23, y: 4.56 };

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::NoChange(_)));

            let old = Foo::A { x: 1.23, y: 4.56 };
            let new = Foo::B { x: 1.23, y: 4.56 };

            let diff = old.diff(&new).unwrap();
            assert!(matches!(
                diff,
                Diff::Modified(DiffType::Enum(EnumDiff::Swapped(..)))
            ));

            let old = Foo::A { x: 1.23, y: 4.56 };
            let new = Bar::A { x: 1.23, y: 4.56 };

            let diff = old.diff(&new).unwrap();
            assert!(matches!(diff, Diff::Replaced(..)));

            let old = Foo::A { x: 1.23, y: 4.56 };
            let new = Foo::A { x: 1.23, y: 7.89 };

            let diff = old.diff(&new).unwrap();
            if let Diff::Modified(modified) = diff {
                if let DiffType::Enum(enum_diff) = modified {
                    if let EnumDiff::Struct(struct_diff) = enum_diff {
                        let mut fields = struct_diff.field_iter();

                        assert!(matches!(fields.next(), Some(("x", Diff::NoChange(_)))));
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
        }
    }
}
