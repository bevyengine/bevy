mod access;
mod parse;

use std::fmt;

use crate::Reflect;
use access::Access;
use parse::PathParser;
use thiserror::Error;

pub use parse::ParseError;

type PathResult<'a, T> = Result<T, ReflectPathError<'a>>;

/// An error specific to accessing a field/index on a `Reflect`.
#[derive(Debug, PartialEq, Eq, Error)]
#[error(transparent)]
pub struct AccessError<'a>(access::Error<'a>);

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("at {offset} in path specification: {error}")]
    InvalidAccess {
        /// Position in the path string.
        offset: usize,
        error: AccessError<'a>,
    },

    #[error("failed to downcast to the path result to the given type")]
    InvalidDowncast,

    #[error("at {offset} in '{path}': {error}")]
    ParseError {
        /// Position in `path`.
        offset: usize,
        path: &'a str,
        error: ParseError<'a>,
    },
}

/// Something that can be interpreted as a reflection path in [`GetPath`].
pub trait ReflectPath<'a>: Sized {
    /// Gets a reference to the specified element on the given [`Reflect`] object.
    ///
    /// See [`GetPath::reflect_path`] for more details,
    /// see [`element`](Self::element) if you want a typed return value.
    fn reflect_element(self, root: &dyn Reflect) -> PathResult<'a, &dyn Reflect>;

    /// Gets a mutable reference to the specified element on the given [`Reflect`] object.
    ///
    /// See [`GetPath::reflect_path_mut`] for more details.
    fn reflect_element_mut(self, root: &mut dyn Reflect) -> PathResult<'a, &mut dyn Reflect>;

    /// Gets a `&T` to the specified element on the given [`Reflect`] object.
    ///
    /// See [`GetPath::path`] for more details.
    fn element<T: Reflect>(self, root: &dyn Reflect) -> PathResult<'a, &T> {
        self.reflect_element(root).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

    /// Gets a `&mut T` to the specified element on the given [`Reflect`] object.
    ///
    /// See [`GetPath::path_mut`] for more details.
    fn element_mut<T: Reflect>(self, root: &mut dyn Reflect) -> PathResult<'a, &mut T> {
        self.reflect_element_mut(root).and_then(|p| {
            p.downcast_mut::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }
}
impl<'a> ReflectPath<'a> for &'a str {
    fn reflect_element(self, mut root: &dyn Reflect) -> PathResult<'a, &dyn Reflect> {
        for (access, offset) in PathParser::new(self) {
            root = access?.element(root, offset)?;
        }
        Ok(root)
    }
    fn reflect_element_mut(self, mut root: &mut dyn Reflect) -> PathResult<'a, &mut dyn Reflect> {
        for (access, offset) in PathParser::new(self) {
            root = access?.element_mut(root, offset)?;
        }
        Ok(root)
    }
}
/// A trait which allows nested [`Reflect`] values to be retrieved with path strings.
///
/// Using these functions repeatedly with the same string requires parsing the string every time.
/// To avoid this cost, it's recommended to construct a [`ParsedPath`] instead.
///
/// # Syntax
///
/// ## Structs
///
/// Field paths for [`Struct`] elements use the standard Rust field access syntax of
/// dot and field name: `.field_name`.
///
/// Additionally, struct fields may be accessed by their index within the struct's definition.
/// This is accomplished by using the hash symbol (`#`) in place of the standard dot: `#0`.
///
/// Accessing a struct's field by index can speed up fetches at runtime due to the removed
/// need for string matching.
/// And while this can be more performant, it's best to keep in mind the tradeoffs when
/// utilizing such optimizations.
/// For example, this can result in fairly fragile code as the string paths will need to be
/// kept in sync with the struct definitions since the order of fields could be easily changed.
/// Because of this, storing these kinds of paths in persistent storage (i.e. game assets)
/// is strongly discouraged.
///
/// Note that a leading dot (`.`) or hash (`#`) token is implied for the first item in a path,
/// and may therefore be omitted.
///
/// ### Example
/// ```
/// # use bevy_reflect::{GetPath, Reflect};
/// #[derive(Reflect)]
/// struct MyStruct {
///   value: u32
/// }
///
/// let my_struct = MyStruct { value: 123 };
/// // Access via field name
/// assert_eq!(my_struct.path::<u32>(".value").unwrap(), &123);
/// // Access via field index
/// assert_eq!(my_struct.path::<u32>("#0").unwrap(), &123);
/// ```
///
/// ## Tuples and Tuple Structs
///
/// [`Tuple`] and [`TupleStruct`] elements also follow a conventional Rust syntax.
/// Fields are accessed with a dot and the field index: `.0`.
///
/// Note that a leading dot (`.`) token is implied for the first item in a path,
/// and may therefore be omitted.
///
/// ### Example
/// ```
/// # use bevy_reflect::{GetPath, Reflect};
/// #[derive(Reflect)]
/// struct MyTupleStruct(u32);
///
/// let my_tuple_struct = MyTupleStruct(123);
/// assert_eq!(my_tuple_struct.path::<u32>(".0").unwrap(), &123);
/// ```
///
/// ## Lists and Arrays
///
/// [`List`] and [`Array`] elements are accessed with brackets: `[0]`.
///
/// ### Example
/// ```
/// # use bevy_reflect::{GetPath};
/// let my_list: Vec<u32> = vec![1, 2, 3];
/// assert_eq!(my_list.path::<u32>("[2]").unwrap(), &3);
/// ```
///
/// ## Enums
///
/// Pathing for [`Enum`] elements works a bit differently than in normal Rust.
/// Usually, you would need to pattern match an enum, branching off on the desired variants.
/// Paths used by this trait do not have any pattern matching capabilities;
/// instead, they assume the variant is already known ahead of time.
///
/// The syntax used, therefore, depends on the variant being accessed:
/// - Struct variants use the struct syntax (outlined above)
/// - Tuple variants use the tuple syntax (outlined above)
/// - Unit variants have no fields to access
///
/// If the variant cannot be known ahead of time, the path will need to be split up
/// and proper enum pattern matching will need to be handled manually.
///
/// ### Example
/// ```
/// # use bevy_reflect::{GetPath, Reflect};
/// #[derive(Reflect)]
/// enum MyEnum {
///   Unit,
///   Tuple(bool),
///   Struct {
///     value: u32
///   }
/// }
///
/// let tuple_variant = MyEnum::Tuple(true);
/// assert_eq!(tuple_variant.path::<bool>(".0").unwrap(), &true);
///
/// let struct_variant = MyEnum::Struct { value: 123 };
/// // Access via field name
/// assert_eq!(struct_variant.path::<u32>(".value").unwrap(), &123);
/// // Access via field index
/// assert_eq!(struct_variant.path::<u32>("#0").unwrap(), &123);
///
/// // Error: Expected struct variant
/// assert!(matches!(tuple_variant.path::<u32>(".value"), Err(_)));
/// ```
///
/// # Chaining
///
/// Using the aforementioned syntax, path items may be chained one after another
/// to create a full path to a nested element.
///
/// ## Example
/// ```
/// # use bevy_reflect::{GetPath, Reflect};
/// #[derive(Reflect)]
/// struct MyStruct {
///   value: Vec<Option<u32>>
/// }
///
/// let my_struct = MyStruct {
///   value: vec![None, None, Some(123)],
/// };
/// assert_eq!(
///   my_struct.path::<u32>(".value[2].0").unwrap(),
///   &123,
/// );
/// ```
///
/// [`Struct`]: crate::Struct
/// [`Tuple`]: crate::Tuple
/// [`TupleStruct`]: crate::TupleStruct
/// [`List`]: crate::List
/// [`Array`]: crate::Array
/// [`Enum`]: crate::Enum
pub trait GetPath: Reflect {
    /// Returns a reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed reference, use
    /// [`path`][GetPath::path].
    fn reflect_path<'p>(&self, path: impl ReflectPath<'p>) -> PathResult<'p, &dyn Reflect> {
        path.reflect_element(self.as_reflect())
    }

    /// Returns a mutable reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed mutable reference, use
    /// [`path_mut`][GetPath::path_mut].
    fn reflect_path_mut<'p>(
        &mut self,
        path: impl ReflectPath<'p>,
    ) -> PathResult<'p, &mut dyn Reflect> {
        path.reflect_element_mut(self.as_reflect_mut())
    }

    /// Returns a statically typed reference to the value specified by `path`.
    ///
    /// This will automatically handle downcasting to type `T`.
    /// The downcast will fail if this value is not of type `T`
    /// (which may be the case when using dynamic types like [`DynamicStruct`]).
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    fn path<'p, T: Reflect>(&self, path: impl ReflectPath<'p>) -> PathResult<'p, &T> {
        path.element(self.as_reflect())
    }

    /// Returns a statically typed mutable reference to the value specified by `path`.
    ///
    /// This will automatically handle downcasting to type `T`.
    /// The downcast will fail if this value is not of type `T`
    /// (which may be the case when using dynamic types like [`DynamicStruct`]).
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    fn path_mut<'p, T: Reflect>(&mut self, path: impl ReflectPath<'p>) -> PathResult<'p, &mut T> {
        path.element_mut(self.as_reflect_mut())
    }
}

// Implement `GetPath` for `dyn Reflect`
impl<T: Reflect + ?Sized> GetPath for T {}

/// A pre-parsed path to an element within a type.
///
/// This struct may be used like [`GetPath`] but removes the cost of parsing the path
/// string at each element access.
///
/// It's recommended to use this in place of `GetPath` when the path string is
/// unlikely to be changed and will be accessed repeatedly.
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ParsedPath(
    /// This is the boxed slice of pre-parsed accesses.
    ///
    /// Each item in the slice contains the access along with the character
    /// index of the start of the access within the parsed path string.
    ///
    /// The index is mainly used for more helpful error reporting.
    Box<[(Access<'static>, usize)]>,
);

impl ParsedPath {
    /// Parses a [`ParsedPath`] from a string.
    ///
    /// Returns an error if the string does not represent a valid path to an element.
    ///
    /// The exact format for path strings can be found in the documentation for [`GetPath`].
    /// In short, though, a path consists of one or more chained accessor strings.
    /// These are:
    /// - Named field access (`.field`)
    /// - Unnamed field access (`.1`)
    /// - Field index access (`#0`)
    /// - Sequence access (`[2]`)
    ///
    /// # Example
    /// ```
    /// # use bevy_reflect::{ParsedPath, Reflect, ReflectPath};
    /// #[derive(Reflect)]
    /// struct Foo {
    ///   bar: Bar,
    /// }
    ///
    /// #[derive(Reflect)]
    /// struct Bar {
    ///   baz: Baz,
    /// }
    ///
    /// #[derive(Reflect)]
    /// struct Baz(f32, Vec<Option<u32>>);
    ///
    /// let foo = Foo {
    ///   bar: Bar {
    ///     baz: Baz(3.14, vec![None, None, Some(123)])
    ///   },
    /// };
    ///
    /// let parsed_path = ParsedPath::parse("bar#0.1[2].0").unwrap();
    /// // Breakdown:
    /// //   "bar" - Access struct field named "bar"
    /// //   "#0" - Access struct field at index 0
    /// //   ".1" - Access tuple struct field at index 1
    /// //   "[2]" - Access list element at index 2
    /// //   ".0" - Access tuple variant field at index 0
    ///
    /// assert_eq!(parsed_path.element::<u32>(&foo).unwrap(), &123);
    /// ```
    ///
    pub fn parse(string: &str) -> PathResult<Self> {
        let mut parts = Vec::new();
        for (access, offset) in PathParser::new(string) {
            parts.push((access?.into_owned(), offset));
        }
        Ok(Self(parts.into_boxed_slice()))
    }

    /// Similar to [`Self::parse`] but only works on `&'static str`
    /// and does not allocate per named field.
    pub fn parse_static(string: &'static str) -> PathResult<Self> {
        let mut parts = Vec::new();
        for (access, offset) in PathParser::new(string) {
            parts.push((access?, offset));
        }
        Ok(Self(parts.into_boxed_slice()))
    }
}
impl<'a> ReflectPath<'a> for &'a ParsedPath {
    fn reflect_element(self, mut root: &dyn Reflect) -> PathResult<'a, &dyn Reflect> {
        for (access, offset) in &*self.0 {
            root = access.element(root, *offset)?;
        }
        Ok(root)
    }
    fn reflect_element_mut(self, mut root: &mut dyn Reflect) -> PathResult<'a, &mut dyn Reflect> {
        for (access, offset) in &*self.0 {
            root = access.element_mut(root, *offset)?;
        }
        Ok(root)
    }
}

impl fmt::Display for ParsedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (access, _) in self.0.iter() {
            write!(f, "{access}")?;
        }
        Ok(())
    }
}
#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::*;
    use access::TypeShape;

    #[derive(Reflect)]
    struct A {
        w: usize,
        x: B,
        y: Vec<C>,
        z: D,
        unit_variant: F,
        tuple_variant: F,
        struct_variant: F,
        array: [i32; 3],
        tuple: (bool, f32),
    }

    #[derive(Reflect)]
    struct B {
        foo: usize,
        łørđ: C,
    }

    #[derive(Reflect)]
    struct C {
        mосква: f32,
    }

    #[derive(Reflect)]
    struct D(E);

    #[derive(Reflect)]
    struct E(f32, usize);

    #[derive(Reflect, PartialEq, Debug)]
    enum F {
        Unit,
        Tuple(u32, u32),
        Şķràźÿ { 東京: char },
    }

    fn a_sample() -> A {
        A {
            w: 1,
            x: B {
                foo: 10,
                łørđ: C { mосква: 3.14 },
            },
            y: vec![C { mосква: 1.0 }, C { mосква: 2.0 }],
            z: D(E(10.0, 42)),
            unit_variant: F::Unit,
            tuple_variant: F::Tuple(123, 321),
            struct_variant: F::Şķràźÿ { 東京: 'm' },
            array: [86, 75, 309],
            tuple: (true, 1.23),
        }
    }

    fn access_field(field: &'static str) -> Access {
        Access::Field(field.into())
    }

    type StaticError = ReflectPathError<'static>;

    fn invalid_access(offset: usize, actual: TypeShape, expected: TypeShape) -> StaticError {
        let error = AccessError(access::Error::Type { actual, expected });
        ReflectPathError::InvalidAccess { offset, error }
    }

    #[test]
    fn parsed_path_parse() {
        assert_eq!(
            &*ParsedPath::parse("w").unwrap().0,
            &[(access_field("w"), 1)]
        );
        assert_eq!(
            &*ParsedPath::parse("x.foo").unwrap().0,
            &[(access_field("x"), 1), (access_field("foo"), 2)]
        );
        assert_eq!(
            &*ParsedPath::parse("x.łørđ.mосква").unwrap().0,
            &[
                (access_field("x"), 1),
                (access_field("łørđ"), 2),
                (access_field("mосква"), 10)
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("y[1].mосква").unwrap().0,
            &[
                (access_field("y"), 1),
                (Access::ListIndex(1), 2),
                (access_field("mосква"), 5)
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("z.0.1").unwrap().0,
            &[
                (access_field("z"), 1),
                (Access::TupleIndex(0), 2),
                (Access::TupleIndex(1), 4),
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("x#0").unwrap().0,
            &[(access_field("x"), 1), (Access::FieldIndex(0), 2)]
        );
        assert_eq!(
            &*ParsedPath::parse("x#0#1").unwrap().0,
            &[
                (access_field("x"), 1),
                (Access::FieldIndex(0), 2),
                (Access::FieldIndex(1), 4)
            ]
        );
    }

    #[test]
    fn parsed_path_get_field() {
        let a = a_sample();

        let b = ParsedPath::parse("w").unwrap();
        let c = ParsedPath::parse("x.foo").unwrap();
        let d = ParsedPath::parse("x.łørđ.mосква").unwrap();
        let e = ParsedPath::parse("y[1].mосква").unwrap();
        let f = ParsedPath::parse("z.0.1").unwrap();
        let g = ParsedPath::parse("x#0").unwrap();
        let h = ParsedPath::parse("x#1#0").unwrap();
        let i = ParsedPath::parse("unit_variant").unwrap();
        let j = ParsedPath::parse("tuple_variant.1").unwrap();
        let k = ParsedPath::parse("struct_variant.東京").unwrap();
        let l = ParsedPath::parse("struct_variant#0").unwrap();
        let m = ParsedPath::parse("array[2]").unwrap();
        let n = ParsedPath::parse("tuple.1").unwrap();

        for _ in 0..30 {
            assert_eq!(*b.element::<usize>(&a).unwrap(), 1);
            assert_eq!(*c.element::<usize>(&a).unwrap(), 10);
            assert_eq!(*d.element::<f32>(&a).unwrap(), 3.14);
            assert_eq!(*e.element::<f32>(&a).unwrap(), 2.0);
            assert_eq!(*f.element::<usize>(&a).unwrap(), 42);
            assert_eq!(*g.element::<usize>(&a).unwrap(), 10);
            assert_eq!(*h.element::<f32>(&a).unwrap(), 3.14);
            assert_eq!(*i.element::<F>(&a).unwrap(), F::Unit);
            assert_eq!(*j.element::<u32>(&a).unwrap(), 321);
            assert_eq!(*k.element::<char>(&a).unwrap(), 'm');
            assert_eq!(*l.element::<char>(&a).unwrap(), 'm');
            assert_eq!(*m.element::<i32>(&a).unwrap(), 309);
            assert_eq!(*n.element::<f32>(&a).unwrap(), 1.23);
        }
    }

    #[test]
    fn reflect_array_behaves_like_list() {
        #[derive(Reflect)]
        struct A {
            list: Vec<u8>,
            array: [u8; 10],
        }

        let a = A {
            list: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            array: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        };

        assert_eq!(*a.path::<u8>("list[5]").unwrap(), 5);
        assert_eq!(*a.path::<u8>("array[5]").unwrap(), 5);
        assert_eq!(*a.path::<u8>("list[0]").unwrap(), 0);
        assert_eq!(*a.path::<u8>("array[0]").unwrap(), 0);
    }

    #[test]
    fn reflect_array_behaves_like_list_mut() {
        #[derive(Reflect)]
        struct A {
            list: Vec<u8>,
            array: [u8; 10],
        }

        let mut a = A {
            list: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
            array: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        };

        assert_eq!(*a.path_mut::<u8>("list[5]").unwrap(), 5);
        assert_eq!(*a.path_mut::<u8>("array[5]").unwrap(), 5);

        *a.path_mut::<u8>("list[5]").unwrap() = 10;
        *a.path_mut::<u8>("array[5]").unwrap() = 10;

        assert_eq!(*a.path_mut::<u8>("list[5]").unwrap(), 10);
        assert_eq!(*a.path_mut::<u8>("array[5]").unwrap(), 10);
    }

    #[test]
    fn reflect_path() {
        let mut a = a_sample();

        assert_eq!(*a.path::<usize>("w").unwrap(), 1);
        assert_eq!(*a.path::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.path::<f32>("x.łørđ.mосква").unwrap(), 3.14);
        assert_eq!(*a.path::<f32>("y[1].mосква").unwrap(), 2.0);
        assert_eq!(*a.path::<usize>("z.0.1").unwrap(), 42);
        assert_eq!(*a.path::<usize>("x#0").unwrap(), 10);
        assert_eq!(*a.path::<f32>("x#1#0").unwrap(), 3.14);

        assert_eq!(*a.path::<F>("unit_variant").unwrap(), F::Unit);
        assert_eq!(*a.path::<u32>("tuple_variant.1").unwrap(), 321);
        assert_eq!(*a.path::<char>("struct_variant.東京").unwrap(), 'm');
        assert_eq!(*a.path::<char>("struct_variant#0").unwrap(), 'm');

        assert_eq!(*a.path::<i32>("array[2]").unwrap(), 309);

        assert_eq!(*a.path::<f32>("tuple.1").unwrap(), 1.23);
        *a.path_mut::<f32>("tuple.1").unwrap() = 3.21;
        assert_eq!(*a.path::<f32>("tuple.1").unwrap(), 3.21);

        *a.path_mut::<f32>("y[1].mосква").unwrap() = 3.0;
        assert_eq!(a.y[1].mосква, 3.0);

        *a.path_mut::<u32>("tuple_variant.0").unwrap() = 1337;
        assert_eq!(a.tuple_variant, F::Tuple(1337, 321));

        assert_eq!(
            a.reflect_path("x.notreal").err().unwrap(),
            ReflectPathError::InvalidAccess {
                offset: 2,
                error: AccessError(access::Error::Access {
                    ty: TypeShape::Struct,
                    access: access_field("notreal"),
                }),
            }
        );

        assert_eq!(
            a.reflect_path("unit_variant.0").err().unwrap(),
            ReflectPathError::InvalidAccess {
                offset: 13,
                error: AccessError(access::Error::Enum {
                    actual: TypeShape::Unit,
                    expected: TypeShape::Tuple
                }),
            }
        );
        assert_eq!(
            a.reflect_path("x[0]").err().unwrap(),
            invalid_access(2, TypeShape::Struct, TypeShape::List)
        );
        assert_eq!(
            a.reflect_path("y.x").err().unwrap(),
            invalid_access(2, TypeShape::List, TypeShape::Struct)
        );
    }

    #[test]
    fn accept_leading_tokens() {
        assert_eq!(
            &*ParsedPath::parse(".w").unwrap().0,
            &[(access_field("w"), 1)]
        );
        assert_eq!(
            &*ParsedPath::parse("#0.foo").unwrap().0,
            &[(Access::FieldIndex(0), 1), (access_field("foo"), 3)]
        );
        assert_eq!(
            &*ParsedPath::parse(".5").unwrap().0,
            &[(Access::TupleIndex(5), 1)]
        );
        assert_eq!(
            &*ParsedPath::parse("[0].łørđ").unwrap().0,
            &[(Access::ListIndex(0), 1), (access_field("łørđ"), 4)]
        );
    }
}
