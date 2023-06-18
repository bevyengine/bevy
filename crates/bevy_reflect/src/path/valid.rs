use core::fmt;
use std::any::{self, TypeId};
use std::marker::PhantomData;

use thiserror::Error;

use super::{Access, ParsedPath};
use crate::{ArrayInfo, NamedField, Reflect, TypeInfo, TypeRegistry, Typed, UnnamedField};

fn out_of_message(len: Option<usize>) -> String {
    if let Some(type_len) = len {
        format!(" out of {type_len} elements")
    } else {
        String::new()
    }
}
const fn access_name(access: &Access) -> &'static str {
    match access {
        Access::Field(_) => "field",
        Access::FieldIndex(_) => "field by index",
        Access::TupleIndex(_) => "tuple field",
        Access::ListIndex(_) => "index",
    }
}
const fn type_info_name(info: &TypeInfo) -> &'static str {
    match info {
        TypeInfo::Struct(_) => "struct",
        TypeInfo::TupleStruct(_) => "tuple struct",
        TypeInfo::Tuple(_) => "tuple",
        TypeInfo::List(_) => "list",
        TypeInfo::Array(_) => "array",
        TypeInfo::Map(_) => "map",
        TypeInfo::Enum(_) => "enum",
        TypeInfo::Value(_) => "value",
    }
}

#[derive(Debug, Error)]
#[error("In {path} on type {source_type} {error}")]
pub struct InvalidPath {
    error: PositionedPathError,
    source_type: &'static str,
    pub path: ParsedPath,
}
impl InvalidPath {
    fn new<Src>(error: PositionedPathError, path: ParsedPath) -> Self {
        InvalidPath {
            source_type: any::type_name::<Src>(),
            error,
            path,
        }
    }
    fn bad_target<Src, Trgt>(got_type: &'static str, path: ParsedPath) -> Self {
        let error = PositionedPathError::BadTarget {
            expected: any::type_name::<Trgt>(),
            got: got_type,
        };
        InvalidPath::new::<Src>(error, path)
    }
}
#[derive(Debug, Error)]
enum PositionedPathError {
    #[error("At column {err_position}: {error}")]
    Access {
        error: PathError,
        err_position: usize,
    },
    #[error("The given path has type '{got}', but it was expected to have type '{expected}'")]
    BadTarget {
        expected: &'static str,
        got: &'static str,
    },
}
impl PositionedPathError {
    fn with_path<Src>(self, path: ParsedPath) -> InvalidPath {
        InvalidPath::new::<Src>(self, path)
    }
}

#[derive(Debug, Error)]
enum PathError {
    #[error(
        "Can't access {} '{access}'{} in {got} {got_type}.",
        access_name(access),
        out_of_message(*type_len),
    )]
    NoSuchField {
        access: Access,
        got: &'static str,
        got_type: &'static str,
        type_len: Option<usize>,
    },
    #[error(
        "Can't access {} '{access}' of a {got} {got_type}. '{access}' can only read {expected}.",
        access_name(access)
    )]
    WrongType {
        access: Access,
        expected: &'static str,
        got: &'static str,
        got_type: &'static str,
    },
    #[error(
        "{queried_type} is not in the registry. Path validation only works with registered types."
    )]
    NotRegistered { queried_type: &'static str },
}
impl PathError {
    fn with_position(self, err_position: usize) -> PositionedPathError {
        PositionedPathError::Access {
            error: self,
            err_position,
        }
    }
}

type InfoResult = Result<&'static TypeInfo, PathError>;

fn item_info(registry: &TypeRegistry, info: &ArrayInfo) -> InfoResult {
    let queried_type = info.item_type_name();
    registry
        .get_type_info(info.item_type_id())
        .ok_or(PathError::NotRegistered { queried_type })
}
fn unnamed_info(registry: &TypeRegistry, field: &UnnamedField) -> InfoResult {
    let queried_type = field.type_name();
    registry
        .get_type_info(field.type_id())
        .ok_or(PathError::NotRegistered { queried_type })
}
fn named_info(registry: &TypeRegistry, field: &NamedField) -> InfoResult {
    let queried_type = field.type_name();
    registry
        .get_type_info(field.type_id())
        .ok_or(PathError::NotRegistered { queried_type })
}

/// Only accepts `TypeInfo::{Struct,TupleStruct,Tuple}` with given field index,
/// returns `TypeInfo` of the field in question.
fn check_index_field<'a>(registry: &TypeRegistry, index: usize, info: &'a TypeInfo) -> InfoResult {
    let no_field = |type_len| PathError::NoSuchField {
        access: Access::FieldIndex(index),
        got: type_info_name(info),
        got_type: info.type_name(),
        type_len: Some(type_len),
    };
    match info {
        TypeInfo::Struct(info) => {
            let field = info.field_at(index).ok_or(no_field(info.field_len()))?;
            named_info(registry, field)
        }
        TypeInfo::TupleStruct(info) => {
            let field = info.field_at(index).ok_or(no_field(info.field_len()))?;
            unnamed_info(registry, field)
        }
        TypeInfo::Tuple(info) => {
            let field = info.field_at(index).ok_or(no_field(info.field_len()))?;
            unnamed_info(registry, field)
        }
        _ => Err(PathError::WrongType {
            access: Access::FieldIndex(index),
            expected: "struct, tuple struct or tuple",
            got: type_info_name(info),
            got_type: info.type_name(),
        }),
    }
}
/// Only accepts `Struct` with given field, returns `TypeInfo` with the field
/// in question, returns the field's `TypeInfo`.
fn check_field(registry: &TypeRegistry, field: &str, info: &TypeInfo) -> InfoResult {
    let no_field = || PathError::NoSuchField {
        access: Access::Field(field.to_string()),
        got: type_info_name(info),
        got_type: info.type_name(),
        type_len: None,
    };
    match info {
        TypeInfo::Struct(info) => {
            let field = info.field(field).ok_or_else(no_field)?;
            named_info(registry, field)
        }
        _ => Err(PathError::WrongType {
            access: Access::Field(field.to_string()),
            expected: "struct",
            got: type_info_name(info),
            got_type: info.type_name(),
        }),
    }
}
/// Only accepts if `info` represents a fixed size `TypeInfo::Array` and
/// array size is greater than `index`, returns `TypeInfo` of the array element.
fn check_index(registry: &TypeRegistry, index: usize, info: &TypeInfo) -> InfoResult {
    match info {
        TypeInfo::Array(array_info) => {
            let type_len = array_info.capacity();
            if type_len <= index {
                return Err(PathError::NoSuchField {
                    access: Access::ListIndex(index),
                    got: type_info_name(info),
                    got_type: info.type_name(),
                    type_len: Some(type_len),
                });
            }
            item_info(registry, array_info)
        }
        _ => Err(PathError::WrongType {
            access: Access::ListIndex(index),
            expected: "array",
            got: type_info_name(info),
            got_type: info.type_name(),
        }),
    }
}
/// Only accepts if `info` represents a `TypeInfo::{Tuple,TupleStruct}` and
/// tuple size is greater than `index`, returns `TypeInfo` of the tuple field.
fn check_tuple_index(registry: &TypeRegistry, index: usize, info: &TypeInfo) -> InfoResult {
    let no_field = |type_len| PathError::NoSuchField {
        access: Access::TupleIndex(index),
        got: type_info_name(info),
        got_type: info.type_name(),
        type_len: Some(type_len),
    };
    match info {
        TypeInfo::TupleStruct(info) => {
            let field = info.field_at(index).ok_or(no_field(info.field_len()))?;
            unnamed_info(registry, field)
        }
        TypeInfo::Tuple(info) => {
            let field = info.field_at(index).ok_or(no_field(info.field_len()))?;
            unnamed_info(registry, field)
        }
        _ => Err(PathError::WrongType {
            access: Access::FieldIndex(index),
            expected: "tuple struct or tuple",
            got: type_info_name(info),
            got_type: info.type_name(),
        }),
    }
}
fn check_access(registry: &TypeRegistry, access: &Access, info: &TypeInfo) -> InfoResult {
    match access {
        Access::Field(field) => check_field(registry, &field, info),
        Access::FieldIndex(index) => check_index_field(registry, *index, info),
        Access::TupleIndex(index) => check_tuple_index(registry, *index, info),
        Access::ListIndex(index) => check_index(registry, *index, info),
    }
}

fn check_path<'a>(
    registry: &TypeRegistry,
    access: &[(Access, usize)],
    info: &'static TypeInfo,
) -> Result<&'static TypeInfo, PositionedPathError> {
    let Some(((access, index), tail)) = access.split_first() else {
        // We reached the end of the access list.
        return Ok(info);
    };
    let info = check_access(registry, access, info).map_err(|err| err.with_position(*index))?;

    check_path(registry, tail, info)
}

/// A typed version of [`ParsedPath`] that can skip most checks.
///
/// Note that indexing operations on variable size containers ([`List`])
/// and enum ([`Enum`]) field access can never be guarenteed to work,
/// so they aren't supported by `ValidPath`.
///
/// See [`ValidPath::new`] to create a `ValidPath`.
///
/// [`Enum`]: crate::Enum
/// [`List`]: crate::List
pub struct ValidPath<Src: ?Sized, Trgt: ?Sized>(ParsedPath, PhantomData<fn(&Src, &Trgt)>);

impl<Src, Trgt> fmt::Debug for ValidPath<Src, Trgt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
impl<Src, Trgt> Clone for ValidPath<Src, Trgt> {
    fn clone(&self) -> Self {
        ValidPath(self.0.clone(), PhantomData)
    }
}

impl<Src: Typed + Reflect, Trgt: Reflect> ValidPath<Src, Trgt> {
    /// Create a typed version of [`ParsedPath`] that can skip some checks,
    /// but will only work on `Src` and `Trgt` types.
    ///
    /// # Errors
    ///
    /// Note that you can read the [`InvalidPath::path`] field to get back ownership
    /// over the [`ParsedPath`] if this fails.
    ///
    /// `validation` will fail and return an `Err` if any of the following is true:
    ///
    /// - The path do not conform to the shape of `Src`. For example, it might
    ///   try to access a field that doesn't exist.
    /// - Any of the field accessed in this `ParsedPath` in `Src` is an [`Enum`]
    /// - Any of the field accessed in this `ParsedPath` in `Src` is a [`List`]
    /// - Any of the field accessed in this `ParsedPath` in `Src`
    ///   is not present in `registry`.
    /// - The target value of this `ParsedPath` is not of type `Trgt`
    ///
    /// [`Enum`]: crate::Enum
    /// [`List`]: crate::List
    pub fn new(registry: &TypeRegistry, path: ParsedPath) -> Result<Self, InvalidPath> {
        // match necessary here because rust can't know we are taking ownership
        // of `path` in closure only if error and return immediately, in case of `map_err`
        let target_info = match check_path(registry, &path.0, Src::type_info()) {
            Ok(value) => value,
            Err(err) => {
                return Err(err.with_path::<Src>(path));
            }
        };
        if target_info.type_id() != TypeId::of::<Trgt>() {
            let target = target_info.type_name();
            return Err(InvalidPath::bad_target::<Src, Trgt>(target, path));
        }
        Ok(ValidPath(path, PhantomData))
    }
    pub fn element<'t>(&self, value: &'t Src) -> &'t Trgt {
        let iter = self.0 .0.iter();
        // SAFETY: Since `ValidPath` can only be constructed through the `new` method,
        // and `new` makes sure we can always get `Trgt` from `Src` with `.0`,
        // this can't possibly fail
        let reflected = iter.fold::<&dyn Reflect, _>(value, |acc, (access, _)| unsafe {
            // NOTE: We don't care about `current_index`, since we can't fail
            access.to_ref().read_element(acc, 0).unwrap_unchecked()
        });
        unsafe { reflected.downcast_ref().unwrap_unchecked() }
    }
    pub fn element_mut<'t>(&self, value: &'t mut Src) -> &'t mut Trgt {
        let iter = self.0 .0.iter();
        // SAFETY: same as above
        let reflected = iter.fold::<&mut dyn Reflect, _>(value, |acc, (access, _)| unsafe {
            access.to_ref().read_element_mut(acc, 0).unwrap_unchecked()
        });
        unsafe { reflected.downcast_mut().unwrap_unchecked() }
    }
}
