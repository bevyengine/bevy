use crate::{ListInfo, MapInfo, Reflect, StructInfo, TupleInfo, TupleStructInfo, UnnamedField};
use std::borrow::{Borrow, Cow};

/// Compile-time type information for various object types
#[derive(Debug, Clone)]
pub enum TypeInfo {
    Struct(StructInfo),
    TupleStruct(TupleStructInfo),
    Tuple(TupleInfo),
    List(ListInfo),
    Map(MapInfo),
    Value(ValueInfo),
    /// Type information for "dynamic" types whose metadata can't be known at compile-time
    ///
    /// This includes structs like [`DynamicStruct`](crate::DynamicStruct) and [`DynamicList`](crate::DynamicList)
    Dynamic(DynamicInfo),
}

/// A container for compile-time info related to general value types, including primitives
#[derive(Debug, Clone)]
pub struct ValueInfo {
    name: Cow<'static, str>,
}

impl ValueInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            name: Cow::Owned(std::any::type_name::<T>().to_string()),
        }
    }

    /// The name of this value
    pub fn name(&self) -> &str {
        self.name.borrow()
    }
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    name: Cow<'static, str>,
}

impl DynamicInfo {
    pub fn new<T: Reflect>() -> Self {
        Self {
            name: Cow::Owned(std::any::type_name::<T>().to_string()),
        }
    }

    /// The name of this value
    pub fn name(&self) -> &str {
        self.name.borrow()
    }
}

/// Create a collection of unnamed fields from an iterator of field type names
pub(crate) fn create_tuple_fields<I: Into<String>, F: IntoIterator<Item = I>>(
    fields: F,
) -> Vec<UnnamedField> {
    fields
        .into_iter()
        .enumerate()
        .map(|(index, field)| UnnamedField::new(index, field.into()))
        .collect()
}
