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

/// A trait which allows nested values to be retrieved with path strings.
///
/// Path strings use Rust syntax:
/// - [`Struct`] items are accessed with a dot and a field name: `.field_name`
/// - [`TupleStruct`] and [`Tuple`] items are accessed with a dot and a number: `.0`
/// - [`List`] items are accessed with brackets: `[0]`
/// - Field indexes within [`Struct`] can also be optionally used instead: `#0` for
///   the first field. This can speed up fetches at runtime (no string matching)
///   but can be much more fragile as code and string paths must be kept in sync since
///   the order of fields could be easily changed. Storing these paths in persistent ///   storage (i.e. game assets) is strongly discouraged.
///
/// If the initial path element is a field of a struct, tuple struct, or tuple,
/// the initial '.' may be omitted.
///
/// For example, given a struct with a field `foo` which is a reflected list of
/// 2-tuples (like a `Vec<(T, U)>`), the path string `foo[3].0` would access tuple
/// element 0 of element 3 of `foo`.
///
/// Using these functions repeatedly with the same string requires parsing the
/// string every time. To avoid this cost, construct a [`FieldPath`] instead.
///
/// [`Struct`]: crate::Struct
/// [`TupleStruct`]: crate::TupleStruct
/// [`Tuple`]: crate::Tuple
/// [`List`]: crate::List
pub trait GetPath {
    /// Returns a reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed reference, use
    /// [`get_path`][GetPath::get_path].
    fn path<'r, 'p>(&'r self, path: &'p str) -> Result<&'r dyn Reflect, ReflectPathError<'p>>;

    /// Returns a mutable reference to the value specified by `path`.
    ///
    /// To retrieve a statically typed mutable reference, use
    /// [`get_path_mut`][GetPath::get_path_mut].
    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>>;

    /// Returns a statically typed reference to the value specified by `path`.
    fn get_path<'r, 'p, T: Reflect>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r T, ReflectPathError<'p>> {
        self.path(path).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

    /// Returns a statically typed mutable reference to the value specified by
    /// `path`.
    fn get_path_mut<'r, 'p, T: Reflect>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut T, ReflectPathError<'p>> {
        self.path_mut(path).and_then(|p| {
            p.downcast_mut::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }
}

impl<T: Reflect> GetPath for T {
    fn path<'r, 'p>(&'r self, path: &'p str) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        (self as &dyn Reflect).path(path)
    }

    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        (self as &mut dyn Reflect).path_mut(path)
    }
}

impl GetPath for dyn Reflect {
    fn path<'r, 'p>(&'r self, path: &'p str) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let mut current: &dyn Reflect = self;
        for (access, current_index) in PathParser::new(path) {
            current = access?.read_field(current, current_index)?;
        }
        Ok(current)
    }

    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current: &mut dyn Reflect = self;
        for (access, current_index) in PathParser::new(path) {
            current = access?.read_field_mut(current, current_index)?;
        }
        Ok(current)
    }
}

/// A path to a field within a type. Can be used like [`GetPath`] functions to get
/// references to the inner fields of a type.
#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct FieldPath(Box<[(Access, usize)]>);

impl FieldPath {
    /// Parses a [`FieldPath`] from a string. For the exact format, see [`GetPath`].
    /// Returns an error if the string does not represent a valid path to a field.
    pub fn parse(string: &str) -> Result<Self, ReflectPathError<'_>> {
        let mut parts = Vec::new();
        for (access, idx) in PathParser::new(string) {
            parts.push((access?.to_owned(), idx));
        }
        Ok(Self(parts.into_boxed_slice()))
    }

    /// Gets a read-only reference of given field.
    /// Returns an error if the path is invalid for the provided type.
    pub fn field<'r, 'p>(
        &'p self,
        root: &'r dyn Reflect,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
        let mut current = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_field(current, *current_index)?;
        }
        Ok(current)
    }

    /// Gets a mutable reference of given field.
    /// Returns an error if the path is invalid for the provided type.
    pub fn field_mut<'r, 'p>(
        &'p mut self,
        root: &'r mut dyn Reflect,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current = root;
        for (access, current_index) in self.0.iter() {
            current = access.to_ref().read_field_mut(current, *current_index)?;
        }
        Ok(current)
    }

    pub fn get_field<'r, 'p, T: Reflect>(
        &'p self,
        root: &'r dyn Reflect,
    ) -> Result<&'r T, ReflectPathError<'p>> {
        self.field(root).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

    pub fn get_field_mut<'r, 'p, T: Reflect>(
        &'p mut self,
        root: &'r mut dyn Reflect,
    ) -> Result<&'r mut T, ReflectPathError<'p>> {
        self.field_mut(root).and_then(|p| {
            p.downcast_mut::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (idx, (access, _)) in self.0.iter().enumerate() {
            match access {
                Access::Field(field) => {
                    if idx != 0 {
                        f.write_str(".")?;
                    }
                    f.write_str(field.as_str())?;
                }
                Access::FieldIndex(index) => {
                    f.write_str("#")?;
                    index.fmt(f)?;
                }
                Access::TupleIndex(index) => {
                    if idx != 0 {
                        f.write_str(".")?;
                    }
                    index.fmt(f)?;
                }
                Access::ListIndex(index) => {
                    f.write_str("[")?;
                    index.fmt(f)?;
                    f.write_str("]")?;
                }
            }
        }
        Ok(())
    }
}

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

    fn read_field<'r>(
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

    fn read_field_mut<'r>(
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
            '.' => {
                self.index += 1;
                return Some(Token::Dot);
            }
            '#' => {
                self.index += 1;
                return Some(Token::CrossHatch);
            }
            '[' => {
                self.index += 1;
                return Some(Token::OpenBracket);
            }
            ']' => {
                self.index += 1;
                return Some(Token::CloseBracket);
            }
            _ => {}
        }

        // we can assume we are parsing an ident now
        for (char_index, character) in self.path[self.index..].chars().enumerate() {
            match character {
                '.' | '#' | '[' | ']' => {
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
                        token: "]",
                    });
                }

                Ok(access)
            }
            Token::CloseBracket => Err(ReflectPathError::UnexpectedToken {
                index: current_index,
                token: "]",
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
    fn field_path_parse() {
        assert_eq!(
            &*FieldPath::parse("w").unwrap().0,
            &[(Access::Field("w".to_string()), 1)]
        );
        assert_eq!(
            &*FieldPath::parse("x.foo").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::Field("foo".to_string()), 2)
            ]
        );
        assert_eq!(
            &*FieldPath::parse("x.bar.baz").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::Field("bar".to_string()), 2),
                (Access::Field("baz".to_string()), 6)
            ]
        );
        assert_eq!(
            &*FieldPath::parse("y[1].baz").unwrap().0,
            &[
                (Access::Field("y".to_string()), 1),
                (Access::ListIndex(1), 2),
                (Access::Field("baz".to_string()), 5)
            ]
        );
        assert_eq!(
            &*FieldPath::parse("z.0.1").unwrap().0,
            &[
                (Access::Field("z".to_string()), 1),
                (Access::TupleIndex(0), 2),
                (Access::TupleIndex(1), 4),
            ]
        );
        assert_eq!(
            &*FieldPath::parse("x#0").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::FieldIndex(0), 2),
            ]
        );
        assert_eq!(
            &*FieldPath::parse("x#0#1").unwrap().0,
            &[
                (Access::Field("x".to_string()), 1),
                (Access::FieldIndex(0), 2),
                (Access::FieldIndex(1), 4)
            ]
        );
    }

    #[test]
    fn field_path_get_field() {
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
        };

        let b = FieldPath::parse("w").unwrap();
        let c = FieldPath::parse("x.foo").unwrap();
        let d = FieldPath::parse("x.bar.baz").unwrap();
        let e = FieldPath::parse("y[1].baz").unwrap();
        let f = FieldPath::parse("z.0.1").unwrap();
        let g = FieldPath::parse("x#0").unwrap();
        let h = FieldPath::parse("x#1#0").unwrap();
        let i = FieldPath::parse("unit_variant").unwrap();
        let j = FieldPath::parse("tuple_variant.1").unwrap();
        let k = FieldPath::parse("struct_variant.value").unwrap();
        let l = FieldPath::parse("struct_variant#0").unwrap();

        for _ in 0..30 {
            assert_eq!(*b.get_field::<usize>(&a).unwrap(), 1);
            assert_eq!(*c.get_field::<usize>(&a).unwrap(), 10);
            assert_eq!(*d.get_field::<f32>(&a).unwrap(), 3.14);
            assert_eq!(*e.get_field::<f32>(&a).unwrap(), 2.0);
            assert_eq!(*f.get_field::<usize>(&a).unwrap(), 42);
            assert_eq!(*g.get_field::<usize>(&a).unwrap(), 10);
            assert_eq!(*h.get_field::<f32>(&a).unwrap(), 3.14);
            assert_eq!(*i.get_field::<F>(&a).unwrap(), F::Unit);
            assert_eq!(*j.get_field::<u32>(&a).unwrap(), 321);
            assert_eq!(*k.get_field::<char>(&a).unwrap(), 'm');
            assert_eq!(*l.get_field::<char>(&a).unwrap(), 'm');
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

        assert_eq!(*a.get_path::<u8>("list[5]").unwrap(), 5);
        assert_eq!(*a.get_path::<u8>("array[5]").unwrap(), 5);
        assert_eq!(*a.get_path::<u8>("list[0]").unwrap(), 0);
        assert_eq!(*a.get_path::<u8>("array[0]").unwrap(), 0);
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

        assert_eq!(*a.get_path_mut::<u8>("list[5]").unwrap(), 5);
        assert_eq!(*a.get_path_mut::<u8>("array[5]").unwrap(), 5);

        *a.get_path_mut::<u8>("list[5]").unwrap() = 10;
        *a.get_path_mut::<u8>("array[5]").unwrap() = 10;

        assert_eq!(*a.get_path_mut::<u8>("list[5]").unwrap(), 10);
        assert_eq!(*a.get_path_mut::<u8>("array[5]").unwrap(), 10);
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
        };

        assert_eq!(*a.get_path::<usize>("w").unwrap(), 1);
        assert_eq!(*a.get_path::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.get_path::<f32>("x.bar.baz").unwrap(), 3.14);
        assert_eq!(*a.get_path::<f32>("y[1].baz").unwrap(), 2.0);
        assert_eq!(*a.get_path::<usize>("z.0.1").unwrap(), 42);
        assert_eq!(*a.get_path::<usize>("x#0").unwrap(), 10);
        assert_eq!(*a.get_path::<f32>("x#1#0").unwrap(), 3.14);

        assert_eq!(*a.get_path::<F>("unit_variant").unwrap(), F::Unit);
        assert_eq!(*a.get_path::<u32>("tuple_variant.1").unwrap(), 321);
        assert_eq!(*a.get_path::<char>("struct_variant.value").unwrap(), 'm');
        assert_eq!(*a.get_path::<char>("struct_variant#0").unwrap(), 'm');

        *a.get_path_mut::<f32>("y[1].baz").unwrap() = 3.0;
        assert_eq!(a.y[1].baz, 3.0);

        *a.get_path_mut::<u32>("tuple_variant.0").unwrap() = 1337;
        assert_eq!(a.tuple_variant, F::Tuple(1337, 321));

        assert_eq!(
            a.path("x.notreal").err().unwrap(),
            ReflectPathError::InvalidField {
                index: 2,
                field: "notreal"
            }
        );

        assert_eq!(
            a.path("unit_variant.0").err().unwrap(),
            ReflectPathError::ExpectedTupleVariant { index: 13 }
        );

        assert_eq!(
            a.path("x..").err().unwrap(),
            ReflectPathError::ExpectedIdent { index: 2 }
        );

        assert_eq!(
            a.path("x[0]").err().unwrap(),
            ReflectPathError::ExpectedList { index: 2 }
        );

        assert_eq!(
            a.path("y.x").err().unwrap(),
            ReflectPathError::ExpectedStruct { index: 2 }
        );

        assert!(matches!(
            a.path("y[badindex]"),
            Err(ReflectPathError::IndexParseError(_))
        ));
    }
}
