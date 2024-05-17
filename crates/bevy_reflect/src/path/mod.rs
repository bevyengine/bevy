pub mod access;
pub use access::*;

mod error;
pub use error::*;

mod parse;
pub use parse::ParseError;
use parse::PathParser;

use crate::Reflect;
use std::fmt;
use thiserror::Error;

type PathResult<'a, T> = Result<T, ReflectPathError<'a>>;

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    /// An error caused by trying to access a path that's not able to be accessed,
    /// see [`AccessError`] for details.
    #[error(transparent)]
    InvalidAccess(AccessError<'a>),

    /// An error that occurs when a type cannot downcast to a given type.
    #[error("Can't downcast result of access to the given type")]
    InvalidDowncast,

    /// An error caused by an invalid path string that couldn't be parsed.
    #[error("Encountered an error at offset {offset} while parsing `{path}`: {error}")]
    ParseError {
        /// Position in `path`.
        offset: usize,
        /// The path that the error occurred in.
        path: &'a str,
        /// The underlying error.
        error: ParseError<'a>,
    },
}
impl<'a> From<AccessError<'a>> for ReflectPathError<'a> {
    fn from(value: AccessError<'a>) -> Self {
        Self::InvalidAccess(value)
    }
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
            let a = access?;
            root = a.element(root, Some(offset))?;
        }
        Ok(root)
    }
    fn reflect_element_mut(self, mut root: &mut dyn Reflect) -> PathResult<'a, &mut dyn Reflect> {
        for (access, offset) in PathParser::new(self) {
            root = access?.element_mut(root, Some(offset))?;
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
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not provide a reflection path",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
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

/// An [`Access`] combined with an `offset` for more helpful error reporting.
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct OffsetAccess {
    /// The [`Access`] itself.
    pub access: Access<'static>,
    /// A character offset in the string the path was parsed from.
    pub offset: Option<usize>,
}

impl From<Access<'static>> for OffsetAccess {
    fn from(access: Access<'static>) -> Self {
        OffsetAccess {
            access,
            offset: None,
        }
    }
}

/// A pre-parsed path to an element within a type.
///
/// This struct can be constructed manually from its [`Access`]es or with
/// the [parse](ParsedPath::parse) method.
///
/// This struct may be used like [`GetPath`] but removes the cost of parsing the path
/// string at each element access.
///
/// It's recommended to use this in place of [`GetPath`] when the path string is
/// unlikely to be changed and will be accessed repeatedly.
///
/// ## Examples
///
/// Parsing a [`&'static str`](str):
/// ```
/// # use bevy_reflect::ParsedPath;
/// let my_static_string: &'static str = "bar#0.1[2].0";
/// // Breakdown:
/// //   "bar" - Access struct field named "bar"
/// //   "#0" - Access struct field at index 0
/// //   ".1" - Access tuple struct field at index 1
/// //   "[2]" - Access list element at index 2
/// //   ".0" - Access tuple variant field at index 0
/// let my_path = ParsedPath::parse_static(my_static_string);
/// ```
/// Parsing a non-static [`&str`](str):
/// ```
/// # use bevy_reflect::ParsedPath;
/// let my_string = String::from("bar#0.1[2].0");
/// // Breakdown:
/// //   "bar" - Access struct field named "bar"
/// //   "#0" - Access struct field at index 0
/// //   ".1" - Access tuple struct field at index 1
/// //   "[2]" - Access list element at index 2
/// //   ".0" - Access tuple variant field at index 0
/// let my_path = ParsedPath::parse(&my_string);
/// ```
/// Manually constructing a [`ParsedPath`]:
/// ```
/// # use std::borrow::Cow;
/// # use bevy_reflect::access::Access;
/// # use bevy_reflect::ParsedPath;
/// let path_elements = [
///     Access::Field(Cow::Borrowed("bar")),
///     Access::FieldIndex(0),
///     Access::TupleIndex(1),
///     Access::ListIndex(2),
///     Access::TupleIndex(1),
/// ];
/// let my_path = ParsedPath::from(path_elements);
/// ```
///
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct ParsedPath(
    /// This is a vector of pre-parsed [`OffsetAccess`]es.
    pub Vec<OffsetAccess>,
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
            parts.push(OffsetAccess {
                access: access?.into_owned(),
                offset: Some(offset),
            });
        }
        Ok(Self(parts))
    }

    /// Similar to [`Self::parse`] but only works on `&'static str`
    /// and does not allocate per named field.
    pub fn parse_static(string: &'static str) -> PathResult<Self> {
        let mut parts = Vec::new();
        for (access, offset) in PathParser::new(string) {
            parts.push(OffsetAccess {
                access: access?,
                offset: Some(offset),
            });
        }
        Ok(Self(parts))
    }
}
impl<'a> ReflectPath<'a> for &'a ParsedPath {
    fn reflect_element(self, mut root: &dyn Reflect) -> PathResult<'a, &dyn Reflect> {
        for OffsetAccess { access, offset } in &self.0 {
            root = access.element(root, *offset)?;
        }
        Ok(root)
    }
    fn reflect_element_mut(self, mut root: &mut dyn Reflect) -> PathResult<'a, &mut dyn Reflect> {
        for OffsetAccess { access, offset } in &self.0 {
            root = access.element_mut(root, *offset)?;
        }
        Ok(root)
    }
}
impl From<Vec<OffsetAccess>> for ParsedPath {
    fn from(value: Vec<OffsetAccess>) -> Self {
        ParsedPath(value)
    }
}
impl<const N: usize> From<[OffsetAccess; N]> for ParsedPath {
    fn from(value: [OffsetAccess; N]) -> Self {
        ParsedPath(value.to_vec())
    }
}
impl From<Vec<Access<'static>>> for ParsedPath {
    fn from(value: Vec<Access<'static>>) -> Self {
        ParsedPath(
            value
                .into_iter()
                .map(|access| OffsetAccess {
                    access,
                    offset: None,
                })
                .collect(),
        )
    }
}
impl<const N: usize> From<[Access<'static>; N]> for ParsedPath {
    fn from(value: [Access<'static>; N]) -> Self {
        value.to_vec().into()
    }
}

impl fmt::Display for ParsedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for OffsetAccess { access, .. } in &self.0 {
            write!(f, "{access}")?;
        }
        Ok(())
    }
}
impl std::ops::Index<usize> for ParsedPath {
    type Output = OffsetAccess;
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}
impl std::ops::IndexMut<usize> for ParsedPath {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.0[index]
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::*;
    use crate as bevy_reflect;
    use crate::*;

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

    fn offset(access: Access<'static>, offset: usize) -> OffsetAccess {
        OffsetAccess {
            access,
            offset: Some(offset),
        }
    }

    fn access_field(field: &'static str) -> Access {
        Access::Field(field.into())
    }

    type StaticError = ReflectPathError<'static>;

    fn invalid_access(
        offset: usize,
        actual: ReflectKind,
        expected: ReflectKind,
        access: &'static str,
    ) -> StaticError {
        ReflectPathError::InvalidAccess(AccessError {
            kind: AccessErrorKind::IncompatibleTypes { actual, expected },
            access: ParsedPath::parse_static(access).unwrap()[1].access.clone(),
            offset: Some(offset),
        })
    }

    #[test]
    fn parsed_path_parse() {
        assert_eq!(
            ParsedPath::parse("w").unwrap().0,
            &[offset(access_field("w"), 1)]
        );
        assert_eq!(
            ParsedPath::parse("x.foo").unwrap().0,
            &[offset(access_field("x"), 1), offset(access_field("foo"), 2)]
        );
        assert_eq!(
            ParsedPath::parse("x.łørđ.mосква").unwrap().0,
            &[
                offset(access_field("x"), 1),
                offset(access_field("łørđ"), 2),
                offset(access_field("mосква"), 10)
            ]
        );
        assert_eq!(
            ParsedPath::parse("y[1].mосква").unwrap().0,
            &[
                offset(access_field("y"), 1),
                offset(Access::ListIndex(1), 2),
                offset(access_field("mосква"), 5)
            ]
        );
        assert_eq!(
            ParsedPath::parse("z.0.1").unwrap().0,
            &[
                offset(access_field("z"), 1),
                offset(Access::TupleIndex(0), 2),
                offset(Access::TupleIndex(1), 4),
            ]
        );
        assert_eq!(
            ParsedPath::parse("x#0").unwrap().0,
            &[
                offset(access_field("x"), 1),
                offset(Access::FieldIndex(0), 2)
            ]
        );
        assert_eq!(
            ParsedPath::parse("x#0#1").unwrap().0,
            &[
                offset(access_field("x"), 1),
                offset(Access::FieldIndex(0), 2),
                offset(Access::FieldIndex(1), 4)
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
            ReflectPathError::InvalidAccess(AccessError {
                kind: AccessErrorKind::MissingField(ReflectKind::Struct),
                access: access_field("notreal"),
                offset: Some(2),
            })
        );

        assert_eq!(
            a.reflect_path("unit_variant.0").err().unwrap(),
            ReflectPathError::InvalidAccess(AccessError {
                kind: AccessErrorKind::IncompatibleEnumVariantTypes {
                    actual: VariantType::Unit,
                    expected: VariantType::Tuple,
                },
                access: ParsedPath::parse_static("unit_variant.0").unwrap()[1]
                    .access
                    .clone(),
                offset: Some(13),
            })
        );
        assert_eq!(
            a.reflect_path("x[0]").err().unwrap(),
            invalid_access(2, ReflectKind::Struct, ReflectKind::List, "x[0]")
        );
        assert_eq!(
            a.reflect_path("y.x").err().unwrap(),
            invalid_access(2, ReflectKind::List, ReflectKind::Struct, "y.x")
        );
    }

    #[test]
    fn accept_leading_tokens() {
        assert_eq!(
            ParsedPath::parse(".w").unwrap().0,
            &[offset(access_field("w"), 1)]
        );
        assert_eq!(
            ParsedPath::parse("#0.foo").unwrap().0,
            &[
                offset(Access::FieldIndex(0), 1),
                offset(access_field("foo"), 3)
            ]
        );
        assert_eq!(
            ParsedPath::parse(".5").unwrap().0,
            &[offset(Access::TupleIndex(5), 1)]
        );
        assert_eq!(
            ParsedPath::parse("[0].łørđ").unwrap().0,
            &[
                offset(Access::ListIndex(0), 1),
                offset(access_field("łørđ"), 4)
            ]
        );
    }
}
