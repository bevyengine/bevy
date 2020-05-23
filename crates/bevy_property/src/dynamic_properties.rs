use std::{collections::HashMap, borrow::Cow};
use crate::{Properties, Property, PropertyIter};
use serde::{Deserialize, Serialize, ser::SerializeMap, de::{self, MapAccess, Visitor}};

#[derive(Default)]
pub struct DynamicProperties {
    pub type_name: String,
    pub props: Vec<(Cow<'static, str>, Box<dyn Property>)>,
    pub prop_indices: HashMap<Cow<'static, str>, usize>,
}

impl DynamicProperties {
    fn push(&mut self, name: &str, prop: Box<dyn Property>) {
        let name: Cow<'static, str> = Cow::Owned(name.to_string());
        self.props.push((name.clone(), prop));
        self.prop_indices.insert(name, self.props.len());
    }
    pub fn set<T: Property>(&mut self, name: &str, prop: T) {
        if let Some(index) = self.prop_indices.get(name) {
            self.props[*index].1 = Box::new(prop);
        } else {
            self.push(name, Box::new(prop));
        }
    }
    pub fn set_box(&mut self, name: &str, prop: Box<dyn Property>) {
        if let Some(index) = self.prop_indices.get(name) {
            self.props[*index].1 = prop;
        } else {
            self.push(name, prop);
        }
    }
}

impl Properties for DynamicProperties {
    #[inline]
    fn type_name(&self) -> &str {
        &self.type_name
    }
    #[inline]
    fn prop(&self, name: &str) -> Option<&dyn Property> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&*self.props[*index].1)
        } else {
            None
        }
    }

    #[inline]
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Property> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&mut *self.props[*index].1)
        } else {
            None
        }
    }

    #[inline]
    fn prop_with_index(&self, index: usize) -> Option<&dyn Property> {
        self.props.get(index).map(|(_i, prop)| &**prop)
    }

    #[inline]
    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Property> {
        self.props.get_mut(index).map(|(_i, prop)| &mut **prop)
    }

    #[inline]
    fn prop_name(&self, index: usize) -> Option<&str> {
        self.props.get(index).map(|(name, _)| name.as_ref())
    }

    #[inline]
    fn prop_len(&self) -> usize {
        self.props.len()
    }

    fn iter_props(&self) -> PropertyIter {
        PropertyIter {
            props: self,
            index: 0,
        }
    }
}

impl Serialize for DynamicProperties {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.props.len()))?;
        state.serialize_entry("type", self.type_name())?;
        for (name, prop) in self.iter_props() {
            state.serialize_entry(name, prop)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for DynamicProperties {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut dynamic_properties = DynamicProperties::default();
        deserializer.deserialize_map(PropMapVisiter {
            dynamic_properties: &mut dynamic_properties,
        })?;

        Ok(dynamic_properties)
    }
}

struct PropMapVisiter<'a> {
    dynamic_properties: &'a mut DynamicProperties,
}

impl<'a, 'de> Visitor<'de> for PropMapVisiter<'a> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("map of properties")
    }

    fn visit_map<V>(self, mut map: V) -> Result<(), V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut type_name: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            if &key == "type" {
                type_name = Some(map.next_value()?); 
            } else {
                let prop = map.next_value()?;
                self.dynamic_properties.set_box(&key, prop);
            }
        }

        let type_name = type_name.ok_or_else(|| de::Error::missing_field("type"))?;
        self.dynamic_properties.type_name = type_name.to_string();
        Ok(())
    }
}

impl<'de> Deserialize<'de> for Box<dyn Property> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(AnyPropVisiter)
    }
}

struct AnyPropVisiter;


impl<'de> Visitor<'de> for AnyPropVisiter {
    type Value = Box<dyn Property>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("property value")
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v)) 
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where E: de::Error {
       Ok(Box::new(v.to_string())) 
    }

    fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut dynamic_properties = DynamicProperties::default();
        // while let Some(key) = map.next_key()? {
        //     let prop = map.next_value()?;
        //     self.dynamic_properties.set_box(key, prop);
        // }
        panic!("aaah");
        // Ok(Box::new(dynamic_properties))
    }
}