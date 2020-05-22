use std::{collections::HashMap, borrow::Cow};
use crate::{Props, Prop, PropIter};
use serde::{Deserialize, Serialize, ser::SerializeMap};

#[derive(Default)]
pub struct DynamicProperties {
    pub type_name: &'static str,
    pub props: Vec<(Cow<'static, str>, Box<dyn Prop>)>,
    pub prop_indices: HashMap<Cow<'static, str>, usize>,
}


impl DynamicProperties {
    fn push(&mut self, name: &str, prop: Box<dyn Prop>) {
        let name: Cow<'static, str> = Cow::Owned(name.to_string());
        self.props.push((name.clone(), prop));
        self.prop_indices.insert(name, self.props.len());
    }
    pub fn set<T: Prop>(&mut self, name: &str, prop: T) {
        if let Some(index) = self.prop_indices.get(name) {
            self.props[*index].1 = Box::new(prop);
        } else {
            self.push(name, Box::new(prop));
        }
    }
    pub fn set_box(&mut self, name: &str, prop: Box<dyn Prop>) {
        if let Some(index) = self.prop_indices.get(name) {
            self.props[*index].1 = prop;
        } else {
            self.push(name, prop);
        }
    }
}


impl Props for DynamicProperties {
    #[inline]
    fn type_name(&self) -> &str {
        self.type_name
    }
    #[inline]
    fn prop(&self, name: &str) -> Option<&dyn Prop> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&*self.props[*index].1)
        } else {
            None
        }
    }

    #[inline]
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&mut *self.props[*index].1)
        } else {
            None
        }
    }

    #[inline]
    fn prop_with_index(&self, index: usize) -> Option<&dyn Prop> {
        self.props.get(index).map(|(_i, prop)| &**prop)
    }

    #[inline]
    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Prop> {
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

    fn iter_props(&self) -> PropIter {
        PropIter {
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

impl<'a> Deserialize<'a> for DynamicProperties {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(DynamicProperties::default())
    }
}