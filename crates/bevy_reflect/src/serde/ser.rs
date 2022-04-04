use crate::{
    serde::type_fields, List, Map, Reflect, ReflectRef, Struct, Tuple, TupleStruct, TypeRegistry,
};
use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};

pub enum Serializable<'a> {
    Owned(Box<dyn erased_serde::Serialize + 'a>),
    Borrowed(&'a dyn erased_serde::Serialize),
}

impl<'a> Serializable<'a> {
    #[allow(clippy::should_implement_trait)]
    pub fn borrow(&self) -> &dyn erased_serde::Serialize {
        match self {
            Serializable::Borrowed(serialize) => serialize,
            Serializable::Owned(serialize) => serialize,
        }
    }
}

fn get_serializable<E: serde::ser::Error>(reflect_value: &dyn Reflect) -> Result<Serializable, E> {
    reflect_value.serializable().ok_or_else(|| {
        serde::ser::Error::custom(format_args!(
            "Type '{}' does not support ReflectValue serialization",
            reflect_value.type_name()
        ))
    })
}

pub struct ReflectSerializer<'a> {
    pub value: &'a dyn Reflect,
    pub registry: &'a TypeRegistry,
}

impl<'a> ReflectSerializer<'a> {
    pub fn new(value: &'a dyn Reflect, registry: &'a TypeRegistry) -> Self {
        ReflectSerializer { value, registry }
    }
}

impl<'a> Serialize for ReflectSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.value.reflect_ref() {
            ReflectRef::Struct(value) => StructSerializer {
                struct_value: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::TupleStruct(value) => TupleStructSerializer {
                tuple_struct: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Tuple(value) => TupleSerializer {
                tuple: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::List(value) => ListSerializer {
                list: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Map(value) => MapSerializer {
                map: value,
                registry: self.registry,
            }
            .serialize(serializer),
            ReflectRef::Value(value) => ReflectValueSerializer {
                registry: self.registry,
                value,
            }
            .serialize(serializer),
        }
    }
}

pub struct ReflectValueSerializer<'a> {
    pub registry: &'a TypeRegistry,
    pub value: &'a dyn Reflect,
}

impl<'a> Serialize for ReflectValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.value.type_name())?;
        state.serialize_entry(
            type_fields::VALUE,
            get_serializable::<S::Error>(self.value)?.borrow(),
        )?;
        state.end()
    }
}

pub struct StructSerializer<'a> {
    pub struct_value: &'a dyn Struct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for StructSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.struct_value.type_name())?;
        state.serialize_entry(
            type_fields::STRUCT,
            &StructValueSerializer {
                struct_value: self.struct_value,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct StructValueSerializer<'a> {
    pub struct_value: &'a dyn Struct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for StructValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.struct_value.field_len()))?;
        for (index, value) in self.struct_value.iter_fields().enumerate() {
            let key = self.struct_value.name_at(index).unwrap();
            state.serialize_entry(key, &ReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct TupleStructSerializer<'a> {
    pub tuple_struct: &'a dyn TupleStruct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleStructSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.tuple_struct.type_name())?;
        state.serialize_entry(
            type_fields::TUPLE_STRUCT,
            &TupleStructValueSerializer {
                tuple_struct: self.tuple_struct,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct TupleStructValueSerializer<'a> {
    pub tuple_struct: &'a dyn TupleStruct,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleStructValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.tuple_struct.field_len()))?;
        for value in self.tuple_struct.iter_fields() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct TupleSerializer<'a> {
    pub tuple: &'a dyn Tuple,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.tuple.type_name())?;
        state.serialize_entry(
            type_fields::TUPLE,
            &TupleValueSerializer {
                tuple: self.tuple,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct TupleValueSerializer<'a> {
    pub tuple: &'a dyn Tuple,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for TupleValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.tuple.field_len()))?;
        for value in self.tuple.iter_fields() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}

pub struct MapSerializer<'a> {
    pub map: &'a dyn Map,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for MapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;

        state.serialize_entry(type_fields::TYPE, self.map.type_name())?;
        state.serialize_entry(
            type_fields::MAP,
            &MapValueSerializer {
                map: self.map,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct MapValueSerializer<'a> {
    pub map: &'a dyn Map,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for MapValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (key, value) in self.map.iter() {
            state.serialize_entry(
                &ReflectSerializer::new(key, self.registry),
                &ReflectSerializer::new(value, self.registry),
            )?;
        }
        state.end()
    }
}

pub struct ListSerializer<'a> {
    pub list: &'a dyn List,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ListSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        state.serialize_entry(type_fields::TYPE, self.list.type_name())?;
        state.serialize_entry(
            type_fields::LIST,
            &ListValueSerializer {
                list: self.list,
                registry: self.registry,
            },
        )?;
        state.end()
    }
}

pub struct ListValueSerializer<'a> {
    pub list: &'a dyn List,
    pub registry: &'a TypeRegistry,
}

impl<'a> Serialize for ListValueSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.list.len()))?;
        for value in self.list.iter() {
            state.serialize_element(&ReflectSerializer::new(value, self.registry))?;
        }
        state.end()
    }
}
