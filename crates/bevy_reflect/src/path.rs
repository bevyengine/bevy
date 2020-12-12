use std::num::ParseIntError;

use crate::{Reflect, ReflectMut, ReflectRef};
use thiserror::Error;

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

pub trait GetPath {
    fn path<'r, 'p>(&'r self, path: &'p str) -> Result<&'r dyn Reflect, ReflectPathError<'p>>;
    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>>;

    fn get_path<'r, 'p, T: Reflect>(
        &'r self,
        path: &'p str,
    ) -> Result<&'r T, ReflectPathError<'p>> {
        self.path(path).and_then(|p| {
            p.downcast_ref::<T>()
                .ok_or(ReflectPathError::InvalidDowncast)
        })
    }

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
        let mut index = 0;
        let mut current: &dyn Reflect = self;
        while let Some(token) = next_token(path, &mut index) {
            let current_index = index;
            match token {
                Token::Dot => {
                    if let Some(Token::Ident(value)) = next_token(path, &mut index) {
                        current = read_field(current, value, current_index)?;
                    } else {
                        return Err(ReflectPathError::ExpectedIdent {
                            index: current_index,
                        });
                    }
                }
                Token::OpenBracket => {
                    if let Some(Token::Ident(value)) = next_token(path, &mut index) {
                        match current.reflect_ref() {
                            ReflectRef::List(reflect_list) => {
                                let list_index = value.parse::<usize>()?;
                                let list_item = reflect_list.get(list_index).ok_or(
                                    ReflectPathError::InvalidListIndex {
                                        index: current_index,
                                        list_index,
                                    },
                                )?;
                                current = list_item;
                            }
                            _ => {
                                return Err(ReflectPathError::ExpectedList {
                                    index: current_index,
                                })
                            }
                        }
                    } else {
                        return Err(ReflectPathError::ExpectedIdent {
                            index: current_index,
                        });
                    }

                    if let Some(Token::CloseBracket) = next_token(path, &mut index) {
                    } else {
                        return Err(ReflectPathError::ExpectedToken {
                            index: current_index,
                            token: "]",
                        });
                    }
                }
                Token::CloseBracket => {
                    return Err(ReflectPathError::UnexpectedToken {
                        index: current_index,
                        token: "]",
                    })
                }
                Token::Ident(value) => {
                    current = read_field(current, value, current_index)?;
                }
            }
        }

        Ok(current)
    }

    fn path_mut<'r, 'p>(
        &'r mut self,
        path: &'p str,
    ) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
        let mut index = 0;
        let mut current: &mut dyn Reflect = self;
        while let Some(token) = next_token(path, &mut index) {
            let current_index = index;
            match token {
                Token::Dot => {
                    if let Some(Token::Ident(value)) = next_token(path, &mut index) {
                        current = read_field_mut(current, value, current_index)?;
                    } else {
                        return Err(ReflectPathError::ExpectedIdent {
                            index: current_index,
                        });
                    }
                }
                Token::OpenBracket => {
                    if let Some(Token::Ident(value)) = next_token(path, &mut index) {
                        match current.reflect_mut() {
                            ReflectMut::List(reflect_list) => {
                                let list_index = value.parse::<usize>()?;
                                let list_item = reflect_list.get_mut(list_index).ok_or(
                                    ReflectPathError::InvalidListIndex {
                                        index: current_index,
                                        list_index,
                                    },
                                )?;
                                current = list_item;
                            }
                            _ => {
                                return Err(ReflectPathError::ExpectedStruct {
                                    index: current_index,
                                })
                            }
                        }
                    } else {
                        return Err(ReflectPathError::ExpectedIdent {
                            index: current_index,
                        });
                    }

                    if let Some(Token::CloseBracket) = next_token(path, &mut index) {
                    } else {
                        return Err(ReflectPathError::ExpectedToken {
                            index: current_index,
                            token: "]",
                        });
                    }
                }
                Token::CloseBracket => {
                    return Err(ReflectPathError::UnexpectedToken {
                        index: current_index,
                        token: "]",
                    })
                }
                Token::Ident(value) => {
                    current = read_field_mut(current, value, current_index)?;
                }
            }
        }

        Ok(current)
    }
}

fn read_field<'r, 'p>(
    current: &'r dyn Reflect,
    field: &'p str,
    current_index: usize,
) -> Result<&'r dyn Reflect, ReflectPathError<'p>> {
    match current.reflect_ref() {
        ReflectRef::Struct(reflect_struct) => {
            Ok(reflect_struct
                .field(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                })?)
        }
        ReflectRef::TupleStruct(reflect_struct) => {
            let tuple_index = field.parse::<usize>()?;
            Ok(reflect_struct.field(tuple_index).ok_or(
                ReflectPathError::InvalidTupleStructIndex {
                    index: current_index,
                    tuple_struct_index: tuple_index,
                },
            )?)
        }
        _ => Err(ReflectPathError::ExpectedStruct {
            index: current_index,
        }),
    }
}

fn read_field_mut<'r, 'p>(
    current: &'r mut dyn Reflect,
    field: &'p str,
    current_index: usize,
) -> Result<&'r mut dyn Reflect, ReflectPathError<'p>> {
    match current.reflect_mut() {
        ReflectMut::Struct(reflect_struct) => {
            Ok(reflect_struct
                .field_mut(field)
                .ok_or(ReflectPathError::InvalidField {
                    index: current_index,
                    field,
                })?)
        }
        ReflectMut::TupleStruct(reflect_struct) => {
            let tuple_index = field.parse::<usize>()?;
            Ok(reflect_struct.field_mut(tuple_index).ok_or(
                ReflectPathError::InvalidTupleStructIndex {
                    index: current_index,
                    tuple_struct_index: tuple_index,
                },
            )?)
        }
        _ => Err(ReflectPathError::ExpectedStruct {
            index: current_index,
        }),
    }
}

enum Token<'a> {
    Dot,
    OpenBracket,
    CloseBracket,
    Ident(&'a str),
}

fn next_token<'a>(path: &'a str, index: &mut usize) -> Option<Token<'a>> {
    if *index >= path.len() {
        return None;
    }

    match path[*index..].chars().next().unwrap() {
        '.' => {
            *index += 1;
            return Some(Token::Dot);
        }
        '[' => {
            *index += 1;
            return Some(Token::OpenBracket);
        }
        ']' => {
            *index += 1;
            return Some(Token::CloseBracket);
        }
        _ => {}
    }

    // we can assume we are parsing an ident now
    for (char_index, character) in path[*index..].chars().enumerate() {
        match character {
            '.' | '[' | ']' => {
                let ident = Token::Ident(&path[*index..*index + char_index]);
                *index += char_index;
                return Some(ident);
            }
            _ => {}
        }
    }
    let ident = Token::Ident(&path[*index..]);
    *index = path.len();
    Some(ident)
}

#[cfg(test)]
mod tests {
    use super::GetPath;
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

        #[derive(Reflect)]
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
