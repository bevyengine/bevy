use serde::{ser::SerializeMap, Deserialize, Serialize};
use std::{any::Any, collections::HashMap};

pub struct Test {
    a: usize,
    b: String,
}

impl Props for Test {
    fn prop(&self, name: &str) -> Option<&dyn Prop> {
        match name {
            "a" => Some(&self.a),
            "b" => Some(&self.b),
            _ => None,
        }
    }
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop> {
        match name {
            "a" => Some(&mut self.a),
            "b" => Some(&mut self.b),
            _ => None,
        }
    }
    fn prop_names(&self) -> Vec<&str> {
        static NAMES: &[&str] = &["a", "b"];
        NAMES.to_vec()
    }
}

#[derive(Default)]
pub struct DynamicProps {
    pub props: HashMap<String, Box<dyn Prop>>,
}

impl DynamicProps {
    pub fn set<T: Prop>(&mut self, name: &str, prop: T) {
        self.props.insert(name.to_string(), Box::new(prop));
    }
}

impl Props for DynamicProps {
    fn prop(&self, name: &str) -> Option<&dyn Prop> {
        self.props.get(name).map(|p| &**p)
    }
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop> {
        self.props.get_mut(name).map(|p| &mut **p)
    }
    fn prop_names(&self) -> Vec<&str> {
        self.props.keys().map(|k| k.as_str()).collect::<Vec<&str>>()
    }
}

impl Serialize for DynamicProps {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.props.len()))?;
        for prop_name in self.prop_names() {
            let prop = self.prop(prop_name).unwrap();
            state.serialize_entry(prop_name, prop)?;
        }
        state.end()
        // let mut state = serializer.serialize_struct("dyn", self.props.len())?;
        // {
        //     for prop_name in self.prop_names() {
        //         let prop = self.prop(prop_name).unwrap();
        //         state.serialize_field(strrr, prop)?;
        //     }
        // }
        // state.end()
    }
}

pub trait Props {
    fn prop(&self, name: &str) -> Option<&dyn Prop>;
    fn prop_mut(&mut self, name: &str) -> Option<&mut dyn Prop>;
    fn prop_names(&self) -> Vec<&str>;
    fn apply(&mut self, props: &dyn Props) {
        for prop_name in props.prop_names() {
            self.prop_mut(prop_name)
                .unwrap()
                .set_prop_val(props.prop(prop_name).unwrap().clone());
        }
    }
}

pub trait Prop: erased_serde::Serialize + Send + Sync + Any + 'static {
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn clone(&self) -> Box<dyn Any>;
    fn type_name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

erased_serde::serialize_trait_object!(Prop);

pub trait PropVal {
    fn prop_val<T: 'static>(&self) -> Option<&T>;
    fn set_prop_val<T: 'static>(&mut self, value: T);
    fn set_prop_val_boxed<T: 'static>(&mut self, value: Box<dyn Any>);
}

impl PropVal for dyn Prop {
    fn prop_val<T: 'static>(&self) -> Option<&T> {
        self.any().downcast_ref::<T>()
    }
    fn set_prop_val<T: 'static>(&mut self, value: T) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = value;
        }
    }
    fn set_prop_val_boxed<T: 'static>(&mut self, value: Box<dyn Any>) {
        if let Some(prop) = self.any_mut().downcast_mut::<T>() {
            *prop = *value.downcast::<T>().unwrap();
        }
    }
}

impl<'a> Deserialize<'a> for DynamicProps {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        Ok(DynamicProps::default())
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
    fn clone(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }
}
