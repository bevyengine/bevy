use std::num::ParseIntError;

use thiserror::Error;

use crate::{Reflect, ReflectMut, ReflectRef};

/// An error returned from a failed path string query.
#[derive(Debug, PartialEq, Eq, Error)]
pub enum ReflectPathError<'a> {
    #[error("expected an identifier at index {index}")]
    ExpectedIdent { index: usize },
    #[error("the current struct doesn't have a field with the name `{field}`")]
    InvalidField { index: usize, field: &'a str },
    #[error("the current tuple struct doesn't have a field with the index {tuple_struct_index}")]
    InvalidTupleStructIndex {
        index: usize,
        tuple_struct_index: usize,
    },
    #[error("the current list doesn't have a value at the index {list_index}")]
    InvalidListIndex { index: usize, list_index: usize },
    #[error("the current map doesn't have a value for the key {map_key}")]
    InvalidMapKey { index: usize, map_key: &'a str },
    #[error("encountered an unexpected token `{token}`")]
    UnexpectedToken { index: usize, token: &'a str },
    #[error("expected token `{token}`, but it wasn't there.")]
    ExpectedToken { index: usize, token: &'a str },
    #[error("expected a struct, but found a different reflect value")]
    ExpectedStruct { index: usize },
    #[error("expected a list, but found a different reflect value")]
    ExpectedList { index: usize },
    #[error("expected a map, but found a different reflect value")]
    ExpectedMap { index: usize },
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
                    match next_token(path, &mut index) {
                        Some(Token::Ident(value)) => match current.reflect_ref() {
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
                        },
                        Some(Token::SingleQuote) => {
                            let ident = match_quoted_ident(path, &mut index)?;

                            match current.reflect_ref() {
                                ReflectRef::Map(reflect_map) => {
                                    let list_item = reflect_map.get(&String::from(ident)).ok_or(
                                        ReflectPathError::InvalidMapKey {
                                            index: current_index,
                                            map_key: ident,
                                        },
                                    )?;
                                    current = list_item;
                                }
                                _ => {
                                    return Err(ReflectPathError::ExpectedMap {
                                        index: current_index,
                                    })
                                }
                            }
                        }
                        _ => {
                            return Err(ReflectPathError::ExpectedIdent {
                                index: current_index,
                            });
                        }
                    }

                    if !matches!(next_token(path, &mut index), Some(Token::CloseBracket)) {
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
                Token::SingleQuote => {
                    return Err(ReflectPathError::UnexpectedToken {
                        index: current_index,
                        token: "'",
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
                    match next_token(path, &mut index) {
                        Some(Token::Ident(value)) => match current.reflect_mut() {
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
                                return Err(ReflectPathError::ExpectedList {
                                    index: current_index,
                                })
                            }
                        },
                        Some(Token::SingleQuote) => {
                            let ident = match_quoted_ident(path, &mut index)?;

                            match current.reflect_mut() {
                                ReflectMut::Map(reflect_map) => {
                                    let list_item = reflect_map
                                        .get_mut(&String::from(ident))
                                        .ok_or(ReflectPathError::InvalidMapKey {
                                            index: current_index,
                                            map_key: ident,
                                        })?;
                                    current = list_item;
                                }
                                _ => {
                                    return Err(ReflectPathError::ExpectedMap {
                                        index: current_index,
                                    })
                                }
                            }
                        }
                        _ => {
                            return Err(ReflectPathError::ExpectedIdent {
                                index: current_index,
                            });
                        }
                    }

                    if !matches!(next_token(path, &mut index), Some(Token::CloseBracket)) {
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
                Token::SingleQuote => {
                    return Err(ReflectPathError::UnexpectedToken {
                        index: current_index,
                        token: "'",
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
    SingleQuote,
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
        '\'' => {
            *index += 1;
            return Some(Token::SingleQuote);
        }
        _ => {}
    }

    // we can assume we are parsing an ident now
    for (char_index, character) in path[*index..].chars().enumerate() {
        match character {
            '.' | '[' | ']' | '\'' => {
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

fn match_quoted_ident<'a>(
    path: &'a str,
    index: &mut usize,
) -> Result<&'a str, ReflectPathError<'a>> {
    let start_of_ident_index = *index;
    let ident = if let Some(Token::Ident(ident)) = next_token(path, index) {
        ident
    } else {
        return Err(ReflectPathError::ExpectedIdent {
            index: start_of_ident_index,
        });
    };

    let end_of_ident_index = *index;
    if !matches!(next_token(path, index), Some(Token::SingleQuote)) {
        return Err(ReflectPathError::ExpectedToken {
            index: end_of_ident_index,
            token: "'",
        });
    }

    Ok(ident)
}

#[cfg(test)]
#[allow(clippy::float_cmp, clippy::approx_constant)]
mod tests {
    use bevy_utils::HashMap;

    use crate as bevy_reflect;
    use crate::*;

    use super::GetPath;

    #[derive(Reflect)]
    struct A {
        w: usize,
        x: B,
        y: Vec<C>,
        z: D,
        map: HashMap<String, usize>,
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

    #[test]
    fn reflect_path() {
        let mut map = HashMap::new();
        map.insert(String::from("test"), 42);
        map.insert(String::from("another"), 123);

        let a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            map,
        };

        assert_eq!(*a.get_path::<usize>("w").unwrap(), 1);
        assert_eq!(*a.get_path::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.get_path::<f32>("x.bar.baz").unwrap(), 3.14);
        assert_eq!(*a.get_path::<f32>("y[1].baz").unwrap(), 2.0);
        assert_eq!(*a.get_path::<usize>("z.0.1").unwrap(), 42);
        assert_eq!(*a.get_path::<usize>("map['test']").unwrap(), 42);
    }

    #[test]
    fn reflect_path_mut() {
        let mut map = HashMap::new();
        map.insert(String::from("test"), 42);
        map.insert(String::from("another"), 123);

        let mut a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            map,
        };

        assert_eq!(*a.get_path_mut::<usize>("w").unwrap(), 1);
        assert_eq!(*a.get_path_mut::<usize>("x.foo").unwrap(), 10);
        assert_eq!(*a.get_path_mut::<f32>("x.bar.baz").unwrap(), 3.14);
        assert_eq!(*a.get_path_mut::<f32>("y[1].baz").unwrap(), 2.0);
        assert_eq!(*a.get_path_mut::<usize>("z.0.1").unwrap(), 42);
        assert_eq!(*a.get_path_mut::<usize>("map['test']").unwrap(), 42);

        *a.get_path_mut::<f32>("y[1].baz").unwrap() = 3.0;
        assert_eq!(a.y[1].baz, 3.0);

        *a.get_path_mut::<usize>("w").unwrap() = 2;
        assert_eq!(a.w, 2);

        *a.get_path_mut::<usize>("map['test']").unwrap() = 1337;
        assert_eq!(a.map["test"], 1337);
    }

    #[test]
    fn reflect_path_errors() {
        let mut map = HashMap::new();
        map.insert(String::from("test"), 42);
        map.insert(String::from("another"), 123);

        let a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            map,
        };

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
            a.path("x['key']").err().unwrap(),
            ReflectPathError::ExpectedMap { index: 2 }
        );

        assert_eq!(
            a.path("y.x").err().unwrap(),
            ReflectPathError::ExpectedStruct { index: 2 }
        );

        assert!(matches!(
            a.path("y[badindex]"),
            Err(ReflectPathError::IndexParseError(_))
        ));

        assert_eq!(
            a.path("y['badindex]").err().unwrap(),
            ReflectPathError::ExpectedToken {
                token: "\'",
                index: 11
            }
        );
        assert_eq!(
            a.path("y['']").err().unwrap(),
            ReflectPathError::ExpectedIdent { index: 3 }
        );
    }

    #[test]
    fn reflect_path_mut_errors() {
        let mut map = HashMap::new();
        map.insert(String::from("test"), 42);
        map.insert(String::from("another"), 123);

        let mut a = A {
            w: 1,
            x: B {
                foo: 10,
                bar: C { baz: 3.14 },
            },
            y: vec![C { baz: 1.0 }, C { baz: 2.0 }],
            z: D(E(10.0, 42)),
            map,
        };

        assert_eq!(
            a.path_mut("x.notreal").err().unwrap(),
            ReflectPathError::InvalidField {
                index: 2,
                field: "notreal"
            }
        );

        assert_eq!(
            a.path_mut("x..").err().unwrap(),
            ReflectPathError::ExpectedIdent { index: 2 }
        );

        assert_eq!(
            a.path_mut("x[0]").err().unwrap(),
            ReflectPathError::ExpectedList { index: 2 }
        );

        assert_eq!(
            a.path_mut("x['key']").err().unwrap(),
            ReflectPathError::ExpectedMap { index: 2 }
        );

        assert_eq!(
            a.path_mut("y.x").err().unwrap(),
            ReflectPathError::ExpectedStruct { index: 2 }
        );

        assert!(matches!(
            a.path_mut("y[badindex]"),
            Err(ReflectPathError::IndexParseError(_))
        ));

        assert_eq!(
            a.path_mut("y['badindex]").err().unwrap(),
            ReflectPathError::ExpectedToken {
                token: "\'",
                index: 11
            }
        );
        assert_eq!(
            a.path_mut("y['']").err().unwrap(),
            ReflectPathError::ExpectedIdent { index: 3 }
        );
    }
}
