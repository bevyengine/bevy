use std::num::ParseIntError;

use crate::{Reflect, ReflectMut, ReflectRef};
use thiserror::Error;

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("expected an identifier at the given index")]
    ExpectedIdent { index: usize },
    #[error("the current struct doesn't have a field with the given name")]
    InvalidField { index: usize, field: &'a str },
    #[error("the current tuple struct doesn't have a field with the given index")]
    InvalidTupleStructIndex {
        index: usize,
        tuple_struct_index: usize,
    },
    #[error("the current list doesn't have a value at the given index")]
    InvalidListIndex { index: usize, list_index: usize },
    #[error("encountered an unexpected token")]
    UnexpectedToken { index: usize, token: &'a str },
    #[error("expected a token, but it wasn't there.")]
    ExpectedToken { index: usize, token: &'a str },
    #[error("expected a struct, but found a different reflect value")]
    ExpectedStruct { index: usize },
    #[error("expected a list, but found a different reflect value")]
    ExpectedList { index: usize },
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
///
/// If the initial path element is a field of a struct, tuple struct, or tuple,
/// the initial '.' may be omitted.
///
/// For example, given a struct with a field `foo` which is a reflected list of
/// 2-tuples (like a `Vec<(T, U)>`), the path string `foo[3].0` would access tuple
/// element 0 of element 3 of `foo`.
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
            match access {
                Ok(access) => {
                    current = access.read_field(current, current_index)?;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(current)
    }

    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut current: &mut dyn Reflect = self;
        for (access, current_index) in PathParser::new(path) {
            match access {
                Ok(access) => {
                    current = access.read_field_mut(current, current_index)?;
                }
                Err(err) => return Err(err),
            }
        }
        Ok(current)
    }
}

#[derive(Debug)]
enum Access<'a> {
    Field(&'a str),
    TupleIndex(usize),
    ListIndex(usize),
}

impl<'a> Access<'a> {
    fn read_field<'r>(
        &self,
        current: &'r dyn Reflect,
        current_index: usize,
    ) -> Result<&'r dyn Reflect, ReflectPathError<'a>> {
        println!("{:?}", self);
        match (self, current.reflect_ref()) {
            (Self::Field(field), ReflectRef::Struct(reflect_struct)) => Ok(reflect_struct
                .field(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                })?),
            (Self::TupleIndex(tuple_index), ReflectRef::TupleStruct(reflect_struct)) => {
                Ok(reflect_struct.field(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )?)
            }
            (Self::ListIndex(list_index), ReflectRef::List(reflect_list)) => Ok(reflect_list
                .get(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                })?),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
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
            (Self::Field(field), ReflectMut::Struct(reflect_struct)) => Ok(reflect_struct
                .field_mut(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                })?),
            (Self::TupleIndex(tuple_index), ReflectMut::TupleStruct(reflect_struct)) => {
                Ok(reflect_struct.field_mut(*tuple_index).ok_or(
                    ReflectPathError::InvalidTupleStructIndex {
                        index: current_index,
                        tuple_struct_index: *tuple_index,
                    },
                )?)
            }
            (Self::ListIndex(list_index), ReflectMut::List(reflect_list)) => Ok(reflect_list
                .get_mut(*list_index)
                .ok_or(ReflectPathError::InvalidListIndex {
                    index: current_index,
                    list_index: *list_index,
                })?),
            (Self::ListIndex(_), _) => Err(ReflectPathError::ExpectedList {
                index: current_index,
            }),
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
                '.' | '[' | ']' => {
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

    fn token_to_access(&mut self, token: Token<'a>) -> Result<Access<'a>, ReflectPathError<'a>> {
        let current_index = self.index;
        match token {
            Token::Dot => {
                if let Some(Token::Ident(value)) = self.next_token() {
                    if let Ok(tuple_index) = value.parse::<usize>() {
                        Ok(Access::TupleIndex(tuple_index))
                    } else {
                        Ok(Access::Field(value))
                    }
                } else {
                    Err(ReflectPathError::ExpectedIdent {
                        index: current_index,
                    })
                }
            }
            Token::OpenBracket => {
                let access = if let Some(Token::Ident(value)) = self.next_token() {
                    Access::ListIndex(value.parse::<usize>()?)
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
            Token::Ident(value) => {
                if let Ok(tuple_index) = value.parse::<usize>() {
                    Ok(Access::TupleIndex(tuple_index))
                } else {
                    Ok(Access::Field(value))
                }
            }
        }
    }
}

impl<'a> Iterator for PathParser<'a> {
    type Item = (Result<Access<'a>, ReflectPathError<'a>>, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(token) = self.next_token() {
            let index = self.index;
            Some((self.token_to_access(token), index))
        } else {
            None
        }
    }
}

enum Token<'a> {
    Dot,
    OpenBracket,
    CloseBracket,
    Ident(&'a str),
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use super::GetPath;
    use crate as bevy_reflect;
    use crate::*;
    #[test]
    fn reflect_path() {
        #[derive(Reflect)]
        struct A {
            w: usize,
            x: B,
            y: Vec<C>,
            z: D,
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

        let mut a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
        };

        assert_eq!(*a.get_path::<usize>("w").unwrap(), 1);
        assert_eq!(*a.get_path::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.get_path::<f32>("x.bar.baz").unwrap(), 3.14);
        assert_eq!(*a.get_path::<f32>("y[1].baz").unwrap(), 2.0);
        assert_eq!(*a.get_path::<usize>("z.0.1").unwrap(), 42);

        *a.get_path_mut::<f32>("y[1].baz").unwrap() = 3.0;
        assert_eq!(a.y[1].baz, 3.0);

        assert_eq!(
            a.path("x.notreal").err().unwrap(),
            ReflectPathError::InvalidField {
                index: 2,
                field: "notreal"
            }
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
