use crate::{DynamicProperties, Properties, PropertiesType, Property, PropertyTypeRegistry};
use de::SeqAccess;
use serde::{
    de::{self, DeserializeSeed, MapAccess, Visitor},
    ser::{SerializeMap, SerializeSeq},
    Serialize,
};
use std::{cell::RefCell, rc::Rc};

impl Serialize for DynamicProperties {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.properties_type {
            PropertiesType::Map => MapSerializer::new(self).serialize(serializer),
            PropertiesType::Seq => SeqSerializer::new(self).serialize(serializer),
        }
    }
}

pub struct DynamicPropertiesDeserializer<'a> {
    pub registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for DynamicPropertiesDeserializer<'a> {
    type Value = DynamicProperties;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(DynamicPropertyMapVisiter {
            registry: self.registry,
            current_type_name: self.current_type_name,
        })
    }
}

pub struct DynamicPropertyMapVisiter<'a> {
    registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for DynamicPropertyMapVisiter<'a> {
    type Value = DynamicProperties;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("properties map")
    }

    fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        visit_map(map, self.registry, self.current_type_name)
    }
}

pub struct MapSerializer<'a> {
    pub properties: &'a dyn Properties,
}

impl<'a> MapSerializer<'a> {
    pub fn new(properties: &'a dyn Properties) -> Self {
        MapSerializer { properties }
    }
}

impl<'a> Serialize for MapSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.properties.prop_len()))?;
        state.serialize_entry("type", self.properties.type_name())?;
        for (index, property) in self.properties.iter_props().enumerate() {
            let name = self.properties.prop_name(index).unwrap();
            if property.is_sequence() {
                state.serialize_entry(name, &SeqSerializer { property })?;
            } else {
                state.serialize_entry(name, property.serializable().borrow())?;
            }
        }
        state.end()
    }
}

pub struct SeqSerializer<'a> {
    pub property: &'a dyn Property,
}

impl<'a> SeqSerializer<'a> {
    pub fn new(property: &'a dyn Property) -> Self {
        SeqSerializer { property }
    }
}

impl<'a> Serialize for SeqSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        if let Some(properties) = self.property.as_properties() {
            state.serialize_entry("seq_type", self.property.type_name())?;
            state.serialize_entry("data", &PropertiesSeqSerializer { properties })?;
        } else {
            state.serialize_entry("seq_value_type", self.property.type_name())?;
            state.serialize_entry("data", self.property.serializable().borrow())?;
        }
        state.end()
    }
}

pub struct PropertiesSeqSerializer<'a> {
    pub properties: &'a dyn Properties,
}

impl<'a> Serialize for PropertiesSeqSerializer<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.properties.prop_len()))?;
        for prop in self.properties.iter_props() {
            state.serialize_element(prop.serializable().borrow())?;
        }
        state.end()
    }
}

pub struct PropertyDeserializer<'a> {
    pub registry: &'a PropertyTypeRegistry,
    pub current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PropertyDeserializer<'a> {
    type Value = Box<dyn Property>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AnyPropVisiter {
            property_type_registry: self.registry,
            current_type_name: self.current_type_name,
        })
    }
}

pub struct PropSeqDeserializer<'a> {
    registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for PropSeqDeserializer<'a> {
    type Value = DynamicProperties;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(PropSeqVisiter {
            registry: self.registry,
            current_type_name: self.current_type_name.clone(),
        })
    }
}

pub struct PropSeqVisiter<'a> {
    registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for PropSeqVisiter<'a> {
    type Value = DynamicProperties;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("property value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut dynamic_properties = DynamicProperties::seq();
        while let Some(prop) = seq.next_element_seed(PropertyDeserializer {
            registry: self.registry,
            current_type_name: self.current_type_name.clone(),
        })? {
            dynamic_properties.push(prop, None);
        }
        Ok(dynamic_properties)
    }
}

pub struct MapValueDeserializer<'a> {
    registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> DeserializeSeed<'de> for MapValueDeserializer<'a> {
    type Value = Box<dyn Property>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.current_type_name.borrow().is_some() {
            let registration = {
                let current_type_name = self.current_type_name.borrow();
                let type_name = current_type_name.as_ref().unwrap();
                self.registry.get_short(type_name).ok_or_else(|| {
                    de::Error::custom(format!("TypeRegistration is missing for {}", type_name))
                })?
            };
            let mut erased = erased_serde::Deserializer::erase(deserializer);
            (registration.deserialize)(&mut erased, self.registry)
                .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)
        } else {
            deserializer.deserialize_any(AnyPropVisiter {
                property_type_registry: self.registry,
                current_type_name: self.current_type_name,
            })
        }
    }
}

struct AnyPropVisiter<'a> {
    property_type_registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
}

impl<'a, 'de> Visitor<'de> for AnyPropVisiter<'a> {
    type Value = Box<dyn Property>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("property value")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(Box::new(v.to_string()))
    }

    fn visit_map<V>(self, map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        Ok(Box::new(visit_map(
            map,
            self.property_type_registry,
            self.current_type_name,
        )?))
    }
}

fn visit_map<'a, 'de, V>(
    mut map: V,
    property_type_registry: &'a PropertyTypeRegistry,
    current_type_name: Rc<RefCell<Option<String>>>,
) -> Result<DynamicProperties, V::Error>
where
    V: MapAccess<'de>,
{
    let mut dynamic_properties = DynamicProperties::map();
    let mut type_name: Option<String> = None;
    let mut is_seq = false;
    // TODO: support seq_value_type
    while let Some(key) = map.next_key::<String>()? {
        if key == "type" {
            type_name = Some(map.next_value()?);
        } else if key == "seq_type" {
            type_name = Some(map.next_value()?);
            is_seq = true;
        } else if is_seq {
            if key != "data" {
                return Err(de::Error::custom(
                    "seq_type must be immediately followed by a data field",
                ));
            }
            dynamic_properties = map.next_value_seed(PropSeqDeserializer {
                registry: property_type_registry,
                current_type_name: current_type_name.clone(),
            })?;
            break;
        } else {
            let prop = map.next_value_seed(MapValueDeserializer {
                registry: property_type_registry,
                current_type_name: current_type_name.clone(),
            })?;
            dynamic_properties.set_box(&key, prop);
        }
    }

    let type_name = type_name.ok_or_else(|| de::Error::missing_field("type"))?;
    dynamic_properties.type_name = type_name.to_string();
    Ok(dynamic_properties)
}
