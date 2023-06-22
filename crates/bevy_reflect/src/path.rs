use std::fmt;
use std::num::ParseIntError;

use crate::{Reflect, ReflectMut, ReflectRef, VariantType};
use thiserror::Error;

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("expected an identifier at index {index}")]
    ExpectedIdent { index: usize },
    #[error("the current struct doesn't have a field with the name `{field}`")]
    InvalidField { index: usize, field: &'a str },
    #[error("the current struct doesn't have a field at the given index")]
    InvalidFieldIndex { index: usize, field_index: usize },
    #[error("the current tuple struct doesn't have a field with the index {tuple_struct_index}")]
    InvalidTupleStructIndex {
        index: usize,
        tuple_struct_index: usize,
    },
    #[error("the current tuple doesn't have a field with the index {tuple_index}")]
    InvalidTupleIndex { index: usize, tuple_index: usize },
    #[error("the current struct variant doesn't have a field with the name `{field}`")]
    InvalidStructVariantField { index: usize, field: &'a str },
    #[error("the current tuple variant doesn't have a field with the index {tuple_variant_index}")]
    InvalidTupleVariantIndex {
        index: usize,
        tuple_variant_index: usize,
    },
    #[error("the current list doesn't have a value at the index {list_index}")]
    InvalidListIndex { index: usize, list_index: usize },
    #[error("encountered an unexpected token `{token}`")]
    UnexpectedToken { index: usize, token: &'a str },
    #[error("expected token `{token}`, but it wasn't there.")]
    ExpectedToken { index: usize, token: &'a str },
    #[error("expected a struct, but found a different reflect value")]
    ExpectedStruct { index: usize },
    #[error("expected a list, but found a different reflect value")]
    ExpectedList { index: usize },
    #[error("expected a struct variant, but found a different reflect value")]
    ExpectedStructVariant { index: usize },
    #[error("expected a tuple variant, but found a different reflect value")]
    ExpectedTupleVariant { index: usize },
    #[error("failed to parse a usize")]
    IndexParseError(#[from] ParseIntError),
    #[error("failed to downcast to the path result to the given type")]
    InvalidDowncast,
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
pub trait GetPath {
    /// Returns a reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed reference, use
    /// [`path`][GetPath::path].
    fn reflect_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>>;

    /// Returns a mutable reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed mutable reference, use
    /// [`path_mut`][GetPath::path_mut].
    fn reflect_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>>;

    /// Returns a statically typed reference to the value specified by `path`.
    ///
    /// This will automatically handle downcasting to type `T`.
    /// The downcast will fail if this value is not of type `T`
    /// (which may be the case when using dynamic types like [`DynamicStruct`]).
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    fn path<'r, 'p, T: Reflect>(&'r self, path: &'p str) -> Result<&'r T, ReflectPathError<'p>> {
        self.reflect_path(path).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

    /// Returns a statically typed mutable reference to the value specified by `path`.
    ///
    /// This will automatically handle downcasting to type `T`.
    /// The downcast will fail if this value is not of type `T`
    /// (which may be the case when using dynamic types like [`DynamicStruct`]).
    ///
    /// [`DynamicStruct`]: crate::DynamicStruct
    fn path_mut<'r, 'p, T: Reflect>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut T, ReflectPathError<'p>> {
        self.reflect_path_mut(path).and_then(|p| {
            p.downcast_mut::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }
}

impl<T: Reflect> GetPath for T {
    fn reflect_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        (self as &dyn Reflect).reflect_path(path)
    }

    fn reflect_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        (self as &mut dyn Reflect).reflect_path_mut(path)
    }
}

impl GetPath for dyn Reflect {
    fn reflect_path<'r, 'p>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let mut current: &dyn Reflect = self;
        for (access, current_index) in PathParser::new(path) {
            current = access?.read_element(current, current_index)?;
        }
        Ok(current)
    }

    fn reflect_path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current: &mut dyn Reflect = self;
        for (access, current_index) in PathParser::new(path) {
            current = access?.read_element_mut(current, current_index)?;
        }
        Ok(current)
    }
}

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
    Box<[(Access, usize)]>,
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
    /// # use bevy_reflect::{ParsedPath, Reflect};
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
    pub fn parse(string: &str) -> Result<Self, ReflectPathError<'_>> {
        let mut parts = Vec::new();
        for (access, idx) in PathParser::new(string) {
            parts.push((access?.to_owned(), idx));
        }
        Ok(Self(parts.into_boxed_slice()))
    }

    /// Gets a read-only reference to the specified element on the given [`Reflect`] object.
    ///
    /// Returns an error if the path is invalid for the provided type.
    ///
    /// See [`element_mut`](Self::reflect_element_mut) for a typed version of this method.
    pub fn reflect_element<'r, 'p>(
        &'p self,
        root: &'r dyn Reflect,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let mut current = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_element(current, *current_index)?;
        }
        Ok(current)
    }

    /// Gets a mutable reference to the specified element on the given [`Reflect`] object.
    ///
    /// Returns an error if the path is invalid for the provided type.
    ///
    /// See [`element_mut`](Self::element_mut) for a typed version of this method.
    pub fn reflect_element_mut<'r, 'p>(
        &'p self,
        root: &'r mut dyn Reflect,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_element_mut(current, *current_index)?;
        }
        Ok(current)
    }

    /// Gets a typed, read-only reference to the specified element on the given [`Reflect`] object.
    ///
    /// Returns an error if the path is invalid for the provided type.
    ///
    /// See [`reflect_element`](Self::reflect_element) for an untyped version of this method.
    pub fn element<'r, 'p, T: Reflect>(
        &'p self,
        root: &'r dyn Reflect,
    ) -> Result<&'r T, ReflectPathError<'p>> {
        self.reflect_element(root).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

    /// Gets a typed, read-only reference to the specified element on the given [`Reflect`] object.
    ///
    /// Returns an error if the path is invalid for the provided type.
    ///
    /// See [`reflect_element_mut`](Self::reflect_element_mut) for an untyped version of this method.
    pub fn element_mut<'r, 'p, T: Reflect>(
        &'p self,
        root: &'r mut dyn Reflect,
    ) -> Result<&'r mut T, ReflectPathError<'p>> {
        self.reflect_element_mut(root).and_then(|p| {
            p.downcast_mut::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }
}

impl fmt::Display for ParsedPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (access, _)) in self.0.iter().enumerate() {
            match access {
                Access::Field(field) => {
                    if idx != 0 {
                        Token::DOT.fmt(f)?;
                    }
                    f.write_str(field.as_str())?;
                }
                Access::FieldIndex(index) => {
                    Token::CROSSHATCH.fmt(f)?;
                    index.fmt(f)?;
                }
                Access::TupleIndex(index) => {
                    if idx != 0 {
                        Token::DOT.fmt(f)?;
                    }
                    index.fmt(f)?;
                }
                Access::ListIndex(index) => {
                    Token::OPEN_BRACKET.fmt(f)?;
                    index.fmt(f)?;
                    Token::CLOSE_BRACKET.fmt(f)?;
                }
            }
        }
        Ok(())
    }
}

/// A singular owned element access within a path.
///
/// Can be applied to a `dyn Reflect` to get a reference to the targeted element.
///
/// A path is composed of multiple accesses in sequence.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Access {
    Field(String),
    FieldIndex(usize),
    TupleIndex(usize),
    ListIndex(usize),
}

impl Access {
    fn to_ref(&self) -> AccessRef<'_> {
        match self {
            Self::Field(value) => AccessRef::Field(value),
            Self::FieldIndex(value) => AccessRef::FieldIndex(*value),
            Self::TupleIndex(value) => AccessRef::TupleIndex(*value),
            Self::ListIndex(value) => AccessRef::ListIndex(*value),
        }
    }
}

/// A singular borrowed element access within a path.
///
/// Can be applied to a `dyn Reflect` to get a reference to the targeted element.
///
/// Does not own the backing store it's sourced from.
/// For an owned version, you can convert one to an [`Access`].
#[derive(Debug)]
enum AccessRef<'a> {
    Field(&'a str),
    FieldIndex(usize),
    TupleIndex(usize),
    ListIndex(usize),
}

impl<'a> AccessRef<'a> {
    fn to_owned(&self) -> Access {
        match self {
            Self::Field(value) => Access::Field(value.to_string()),
            Self::FieldIndex(value) => Access::FieldIndex(*value),
            Self::TupleIndex(value) => Access::TupleIndex(*value),
            Self::ListIndex(value) => Access::ListIndex(*value),
        }
    }

    fn read_element<'r>(
        &self,
        current: &'r dyn Reflect,
        current_index: usize,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'a>> {
        match (self, current.reflect_ref()) {
            (Self::Field(field), ReflectRef::Struct(reflect_struct)) => reflect_struct
                .field(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                }),
            (Self::FieldIndex(field_index), ReflectRef::Struct(reflect_struct)) => reflect_struct
                .field_at(*field_index)
                .ok_or(ReflectPathError::InvalidFieldIndex {
                    index: current_index,
                    field_index: *field_index,
                }),
            (Self::TupleIndex(tuple_index), ReflectRef::TupleStruct(reflect_struct)) => {
                reflect_struct.field(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )
            }
            (Self::TupleIndex(tuple_index), ReflectRef::Tuple(reflect_tuple)) => reflect_tuple
                .field(*tuple_index)
                .ok_or(ReflectPathError::InvalidTupleIndex {
                    index: current_index,
                    tuple_index: *tuple_index,
                }),
            (Self::ListIndex(list_index), ReflectRef::List(reflect_list)) => reflect_list
                .get(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(list_index), ReflectRef::Array(reflect_list)) => reflect_list
                .get(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
            (Self::Field(field), ReflectRef::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Struct => {
                        reflect_enum
                            .field(field)
                            .ok_or(ReflectPathError::InvalidField {
                                index: current_index,
                                field,
                            })
                    }
                    _ => Err(ReflectPathError::ExpectedStructVariant {
                        index: current_index,
                    }),
                }
            }
            (Self::FieldIndex(field_index), ReflectRef::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Struct => reflect_enum.field_at(*field_index).ok_or(
                        ReflectPathError::InvalidFieldIndex {
                            index: current_index,
                            field_index: *field_index,
                        },
                    ),
                    _ => Err(ReflectPathError::ExpectedStructVariant {
                        index: current_index,
                    }),
                }
            }
            (Self::TupleIndex(tuple_variant_index), ReflectRef::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Tuple => reflect_enum.field_at(*tuple_variant_index).ok_or(
                        ReflectPathError::InvalidTupleVariantIndex {
                            index: current_index,
                            tuple_variant_index: *tuple_variant_index,
                        },
                    ),
                    _ => Err(ReflectPathError::ExpectedTupleVariant {
                        index: current_index,
                    }),
                }
            }
            _ => Err(ReflectPathError::ExpectedStruct {
                index: current_index,
            }),
        }
    }

    fn read_element_mut<'r>(
        &self,
        current: &'r mut dyn Reflect,
        current_index: usize,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'a>> {
        match (self, current.reflect_mut()) {
            (Self::Field(field), ReflectMut::Struct(reflect_struct)) => reflect_struct
                .field_mut(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                }),
            (Self::FieldIndex(field_index), ReflectMut::Struct(reflect_struct)) => reflect_struct
                .field_at_mut(*field_index)
                .ok_or(ReflectPathError::InvalidFieldIndex {
                    index: current_index,
                    field_index: *field_index,
                }),
            (Self::TupleIndex(tuple_index), ReflectMut::TupleStruct(reflect_struct)) => {
                reflect_struct.field_mut(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )
            }
            (Self::TupleIndex(tuple_index), ReflectMut::Tuple(reflect_tuple)) => reflect_tuple
                .field_mut(*tuple_index)
                .ok_or(ReflectPathError::InvalidTupleIndex {
                    index: current_index,
                    tuple_index: *tuple_index,
                }),
            (Self::ListIndex(list_index), ReflectMut::List(reflect_list)) => reflect_list
                .get_mut(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(list_index), ReflectMut::Array(reflect_list)) => reflect_list
                .get_mut(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                }),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
            (Self::Field(field), ReflectMut::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Struct => {
                        reflect_enum
                            .field_mut(field)
                            .ok_or(ReflectPathError::InvalidField {
                                index: current_index,
                                field,
                            })
                    }
                    _ => Err(ReflectPathError::ExpectedStructVariant {
                        index: current_index,
                    }),
                }
            }
            (Self::FieldIndex(field_index), ReflectMut::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Struct => reflect_enum.field_at_mut(*field_index).ok_or(
                        ReflectPathError::InvalidFieldIndex {
                            index: current_index,
                            field_index: *field_index,
                        },
                    ),
                    _ => Err(ReflectPathError::ExpectedStructVariant {
                        index: current_index,
                    }),
                }
            }
            (Self::TupleIndex(tuple_variant_index), ReflectMut::Enum(reflect_enum)) => {
                match reflect_enum.variant_type() {
                    VariantType::Tuple => reflect_enum.field_at_mut(*tuple_variant_index).ok_or(
                        ReflectPathError::InvalidTupleVariantIndex {
                            index: current_index,
                            tuple_variant_index: *tuple_variant_index,
                        },
                    ),
                    _ => Err(ReflectPathError::ExpectedTupleVariant {
                        index: current_index,
                    }),
                }
            }
            _ => Err(ReflectPathError::ExpectedStruct {
                index: current_index,
            }),
        }
    }
}

struct PathParser<'a> {
    path: &'a str,
    index: usize,
}

impl<'a> PathParser<'a> {
    fn new(path: &'a str) -> Self {
        Self { path, index: 0 }
    }

    fn next_token(&mut self) -> Option<Token<'a>> {
        if self.index >= self.path.len() {
            return None;
        }

        match self.path[self.index..].chars().next().unwrap() {
            Token::DOT => {
                self.index += 1;
                return Some(Token::Dot);
            }
            Token::CROSSHATCH => {
                self.index += 1;
                return Some(Token::CrossHatch);
            }
            Token::OPEN_BRACKET => {
                self.index += 1;
                return Some(Token::OpenBracket);
            }
            Token::CLOSE_BRACKET => {
                self.index += 1;
                return Some(Token::CloseBracket);
            }
            _ => {}
        }

        // we can assume we are parsing an ident now
        for (char_index, character) in self.path[self.index..].chars().enumerate() {
            match character {
                Token::DOT | Token::CROSSHATCH | Token::OPEN_BRACKET | Token::CLOSE_BRACKET => {
                    let ident = Token::Ident(&self.path[self.index..self.index + char_index]);
                    self.index += char_index;
                    return Some(ident);
                }
                _ => {}
            }
        }
        let ident = Token::Ident(&self.path[self.index..]);
        self.index = self.path.len();
        Some(ident)
    }

    fn token_to_access(&mut self, token: Token<'a>) -> Result<AccessRef<'a>, ReflectPathError<'a>> {
        let current_index = self.index;
        match token {
            Token::Dot => {
                if let Some(Token::Ident(value)) = self.next_token() {
                    value
                        .parse::<usize>()
                        .map(AccessRef::TupleIndex)
                        .or(Ok(AccessRef::Field(value)))
                } else {
                    Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    })
                }
            }
            Token::CrossHatch => {
                if let Some(Token::Ident(value)) = self.next_token() {
                    Ok(AccessRef::FieldIndex(value.parse::<usize>()?))
                } else {
                    Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    })
                }
            }
            Token::OpenBracket => {
                let access = if let Some(Token::Ident(value)) = self.next_token() {
                    AccessRef::ListIndex(value.parse::<usize>()?)
                } else {
                    return Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    });
                };

                if !matches!(self.next_token(), Some(Token::CloseBracket)) {
                    return Err(ReflectPathError::ExpectedToken {
                        index: current_index,
                        token: Token::OPEN_BRACKET_STR,
                    });
                }

                Ok(access)
            }
            Token::CloseBracket => Err(ReflectPathError::UnexpectedToken {
                index: current_index,
                token: Token::CLOSE_BRACKET_STR,
            }),
            Token::Ident(value) => value
                .parse::<usize>()
                .map(AccessRef::TupleIndex)
                .or(Ok(AccessRef::Field(value))),
        }
    }
}

impl<'a> Iterator for PathParser<'a> {
    type Item = (Result<AccessRef<'a>, ReflectPathError<'a>>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.next_token()?;
        let index = self.index;
        Some((self.token_to_access(token), index))
    }
}

enum Token<'a> {
    Dot,
    CrossHatch,
    OpenBracket,
    CloseBracket,
    Ident(&'a str),
}

impl<'a> Token<'a> {
    const DOT: char = '.';
    const CROSSHATCH: char = '#';
    const OPEN_BRACKET: char = '[';
    const CLOSE_BRACKET: char = ']';
    const OPEN_BRACKET_STR: &'static str = "[";
    const CLOSE_BRACKET_STR: &'static str = "]";
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
        bar: C,
    }

    #[derive(Reflect, FromReflect)]
    struct C {
        baz: f32,
    }

    #[derive(Reflect)]
    struct D(E);

    #[derive(Reflect)]
    struct E(f32, usize);

    #[derive(Reflect, FromReflect, PartialEq, Debug)]
    enum F {
        Unit,
        Tuple(u32, u32),
        Struct { value: char },
    }

    #[test]
    fn parsed_path_parse() {
        assert_eq!(
            &*ParsedPath::parse("w").unwrap().0,
            &[(Access::Field("w".to_string()), 1)]
        );
        assert_eq!(
            &*ParsedPath::parse("x.foo").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::Field("foo".to_string()), 2)
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("x.bar.baz").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::Field("bar".to_string()), 2),
                (Access::Field("baz".to_string()), 6)
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("y[1].baz").unwrap().0,
            &[
                (Access::Field("y".to_string()), 1),
                (Access::ListIndex(1), 2),
                (Access::Field("baz".to_string()), 5)
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("z.0.1").unwrap().0,
            &[
                (Access::Field("z".to_string()), 1),
                (Access::TupleIndex(0), 2),
                (Access::TupleIndex(1), 4),
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("x#0").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::FieldIndex(0), 2),
            ]
        );
        assert_eq!(
            &*ParsedPath::parse("x#0#1").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::FieldIndex(0), 2),
                (Access::FieldIndex(1), 4)
            ]
        );
    }

    #[test]
    fn parsed_path_get_field() {
        let a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            unit_variant: F::Unit,
            tuple_variant: F::Tuple(123, 321),
            struct_variant: F::Struct { value: 'm' },
            array: [86, 75, 309],
            tuple: (true, 1.23),
        };

        let b = ParsedPath::parse("w").unwrap();
        let c = ParsedPath::parse("x.foo").unwrap();
        let d = ParsedPath::parse("x.bar.baz").unwrap();
        let e = ParsedPath::parse("y[1].baz").unwrap();
        let f = ParsedPath::parse("z.0.1").unwrap();
        let g = ParsedPath::parse("x#0").unwrap();
        let h = ParsedPath::parse("x#1#0").unwrap();
        let i = ParsedPath::parse("unit_variant").unwrap();
        let j = ParsedPath::parse("tuple_variant.1").unwrap();
        let k = ParsedPath::parse("struct_variant.value").unwrap();
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
        let mut a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            unit_variant: F::Unit,
            tuple_variant: F::Tuple(123, 321),
            struct_variant: F::Struct { value: 'm' },
            array: [86, 75, 309],
            tuple: (true, 1.23),
        };

        assert_eq!(*a.path::<usize>("w").unwrap(), 1);
        assert_eq!(*a.path::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.path::<f32>("x.bar.baz").unwrap(), 3.14);
        assert_eq!(*a.path::<f32>("y[1].baz").unwrap(), 2.0);
        assert_eq!(*a.path::<usize>("z.0.1").unwrap(), 42);
        assert_eq!(*a.path::<usize>("x#0").unwrap(), 10);
        assert_eq!(*a.path::<f32>("x#1#0").unwrap(), 3.14);

        assert_eq!(*a.path::<F>("unit_variant").unwrap(), F::Unit);
        assert_eq!(*a.path::<u32>("tuple_variant.1").unwrap(), 321);
        assert_eq!(*a.path::<char>("struct_variant.value").unwrap(), 'm');
        assert_eq!(*a.path::<char>("struct_variant#0").unwrap(), 'm');

        assert_eq!(*a.path::<i32>("array[2]").unwrap(), 309);

        assert_eq!(*a.path::<f32>("tuple.1").unwrap(), 1.23);
        *a.path_mut::<f32>("tuple.1").unwrap() = 3.21;
        assert_eq!(*a.path::<f32>("tuple.1").unwrap(), 3.21);

        *a.path_mut::<f32>("y[1].baz").unwrap() = 3.0;
        assert_eq!(a.y[1].baz, 3.0);

        *a.path_mut::<u32>("tuple_variant.0").unwrap() = 1337;
        assert_eq!(a.tuple_variant, F::Tuple(1337, 321));

        assert_eq!(
            a.reflect_path("x.notreal").err().unwrap(),
            ReflectPathError::InvalidField {
                index: 2,
                field: "notreal"
            }
        );

        assert_eq!(
            a.reflect_path("unit_variant.0").err().unwrap(),
            ReflectPathError::ExpectedTupleVariant { index: 13 }
        );

        assert_eq!(
            a.reflect_path("x..").err().unwrap(),
            ReflectPathError::ExpectedIdent { index: 2 }
        );

        assert_eq!(
            a.reflect_path("x[0]").err().unwrap(),
            ReflectPathError::ExpectedList { index: 2 }
        );

        assert_eq!(
            a.reflect_path("y.x").err().unwrap(),
            ReflectPathError::ExpectedStruct { index: 2 }
        );

        assert!(matches!(
            a.reflect_path("y[badindex]"),
            Err(ReflectPathError::IndexParseError(_))
        ));
    }
}
