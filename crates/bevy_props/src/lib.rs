use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::{any::Any, collections::HashMap, borrow::Cow};

pub struct DynamicScene {
    pub entities: Vec<SceneEntity>,
}

#[derive(Serialize, Deserialize)]
pub struct SceneEntity {
    pub entity: u32,
    pub components: Vec<DynamicProperties>,
}

#[derive(Default)]
pub struct DynamicProperties {
    pub type_name: &'static str,
    pub props: Vec<(Cow<'static, str>, Box<dyn Prop>)>,
    pub prop_indices: HashMap<Cow<'static, str>, usize>,
}

pub struct SerializableProps<'a> {
    pub props: &'a dyn Props,
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
    fn type_name(&self) -> &str {
        self.type_name
    }
    fn prop(&self, name: &str) -> Option<&dyn Prop> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&*self.props[*index].1)
        } else {
            None
        }
    }

    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop> {
        if let Some(index) = self.prop_indices.get(name) {
            Some(&mut *self.props[*index].1)
        } else {
            None
        }
    }

    fn prop_with_index(&self, index: usize) -> Option<&dyn Prop> {
        self.props.get(index).map(|(_i, prop)| &**prop)
    }

    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Prop> {
        self.props.get_mut(index).map(|(_i, prop)| &mut **prop)
    }

    fn prop_name(&self, index: usize) -> Option<&str> {
        self.props.get(index).map(|(name, _)| name.as_ref())
    }

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

impl<'a> Serialize for SerializableProps<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.props.prop_len()))?;
        state.serialize_entry("type", self.props.type_name())?;
        for (name, prop) in self.props.iter_props() {
            state.serialize_entry(name, prop)?;
        }
        state.end()
    }
}

pub trait Props {
    fn type_name(&self) -> &str;
    fn prop(&self, name: &str) -> Option<&dyn Prop>;
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop>;
    fn prop_with_index(&self, index: usize) -> Option<&dyn Prop>;
    fn prop_with_index_mut(&mut self, index: usize) -> Option<&mut dyn Prop>;
    fn prop_name(&self, index: usize) -> Option<&str>;
    fn prop_len(&self) -> usize;
    fn iter_props(&self) -> PropIter;
    fn apply(&mut self, props: &dyn Props) {
        for (name, prop) in props.iter_props() {
            self.prop_mut(name).unwrap().set_val_dyn(prop);
        }
    }
    fn to_dynamic(&self) -> DynamicProperties where Self: 'static {
        let mut dynamic_props = DynamicProperties::default();
        for (name, prop) in self.iter_props() {
            dynamic_props.set_box(name, prop.clone_prop());
        }

        dynamic_props.type_name = std::any::type_name::<Self>();
        dynamic_props
    }
}

pub struct PropIter<'a> {
    props: &'a dyn Props,
    index: usize,
}

impl<'a> PropIter<'a> {
    pub fn new(props: &'a dyn Props) -> Self {
        PropIter {
            props,
            index: 0,
        }
    }
}

impl<'a> Iterator for PropIter<'a> {
    type Item = (&'a str, &'a dyn Prop);
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.props.prop_len() {
            let prop = self.props.prop_with_index(self.index).unwrap();
            let name = self.props.prop_name(self.index).unwrap();
            self.index += 1;
            Some((name, prop))
        } else {
            None
        }
    }
}

pub trait Prop: erased_serde::Serialize + Send + Sync + Any + 'static {
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone_prop(&self) -> Box<dyn Prop>;
    fn set_val_dyn(&mut self, value: &dyn Prop);
}

erased_serde::serialize_trait_object!(Prop);

pub trait PropVal {
    fn val<T: 'static>(&self) -> Option<&T>;
    fn set_val<T: 'static>(&mut self, value: T);
}

pub trait PropsVal {
    fn prop_val<T: 'static>(&self, name: &str) -> Option<&T>;
    fn set_prop_val<T: 'static>(&mut self, name: &str, value: T);
    fn set_prop_val_dyn<T: 'static>(&mut self, name: &str, value: &dyn Prop);
}


impl PropVal for dyn Prop {
    fn val<T: 'static>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }
    fn set_val<T: 'static>(&mut self, value: T) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = value;
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}

impl<P> PropsVal for P where P: Props {
    fn prop_val<T: 'static>(&self, name: &str) -> Option<&T> {
        self.prop(name).and_then(|p| p.any().downcast_ref::<T>())
    }
    fn set_prop_val<T: 'static>(&mut self, name: &str, value: T) {
        if let Some(prop) = self.prop_mut(name).and_then(|p| p.any_mut().downcast_mut::<T>()) {
            *prop = value;
        } else {
            panic!("prop does not exist or is incorrect type: {}", name);
        }
    }
    fn set_prop_val_dyn<T: 'static>(&mut self, name: &str, value: &dyn Prop) {
        if let Some(prop) = self.prop_mut(name) {
            prop.set_val_dyn(value);
        } else {
            panic!("prop does not exist: {}", name);
        }
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

impl<T> Prop for T
where
    T: Clone + Serialize + Send + Sync + Any + 'static,
{
    fn any(&self) -> &dyn Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn clone_prop(&self) -> Box<dyn Prop> {
        Box::new(self.clone())
    }
    fn set_val_dyn(&mut self, value: &dyn Prop) {
        if let Some(prop) = value.any().downcast_ref::<T>() {
            *self = prop.clone();
        } else {
            panic!("prop value is not {}", std::any::type_name::<T>());
        }
    }
}
