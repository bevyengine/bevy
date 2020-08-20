use crate::{DeserializeProperty, Property};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

#[derive(Default)]
pub struct PropertyTypeRegistry {
    registrations: HashMap<String, PropertyTypeRegistration>,
    short_names: HashMap<String, String>,
    ambigous_names: HashSet<String>,
}

impl PropertyTypeRegistry {
    pub fn register<T>(&mut self)
    where
        T: Property + DeserializeProperty,
    {
        let registration = PropertyTypeRegistration::of::<T>();
        self.add_registration(registration);
    }

    fn add_registration(&mut self, registration: PropertyTypeRegistration) {
        let short_name = registration.short_name.to_string();
        if self.short_names.contains_key(&short_name) || self.ambigous_names.contains(&short_name) {
            // name is ambiguous. fall back to long names for all ambiguous types
            self.short_names.remove(&short_name);
            self.ambigous_names.insert(short_name);
        } else {
            self.short_names
                .insert(short_name, registration.name.to_string());
        }
        self.registrations
            .insert(registration.name.to_string(), registration);
    }

    pub fn get(&self, type_name: &str) -> Option<&PropertyTypeRegistration> {
        if let Some(long_name) = self.short_names.get(type_name) {
            self.registrations.get(long_name)
        } else {
            self.registrations.get(type_name)
        }
    }

    pub fn format_type_name(&self, type_name: &str) -> Option<&str> {
        self.get(type_name).map(|registration| {
            if self.short_names.contains_key(&registration.short_name) {
                &registration.short_name
            } else {
                registration.name
            }
        })
    }

    pub fn get_with_short_name(&self, short_type_name: &str) -> Option<&PropertyTypeRegistration> {
        self.short_names
            .get(short_type_name)
            .and_then(|name| self.registrations.get(name))
    }

    pub fn get_with_full_name(&self, type_name: &str) -> Option<&PropertyTypeRegistration> {
        self.registrations.get(type_name)
    }
}

#[derive(Clone)]
pub struct PropertyTypeRegistration {
    pub ty: TypeId,
    deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        property_type_registry: &PropertyTypeRegistry,
    ) -> Result<Box<dyn Property>, erased_serde::Error>,
    pub short_name: String,
    pub name: &'static str,
}

impl PropertyTypeRegistration {
    pub fn of<T: Property + DeserializeProperty>() -> Self {
        let ty = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        Self {
            ty,
            deserialize_fn:
                |deserializer: &mut dyn erased_serde::Deserializer,
                 property_type_registry: &PropertyTypeRegistry| {
                    T::deserialize(deserializer, property_type_registry)
                },
            name: type_name,
            short_name: Self::get_short_name(type_name),
        }
    }

    pub fn get_short_name(full_name: &str) -> String {
        let mut split = full_name.splitn(2, '<');

        // main type
        let mut short_name = split
            .next()
            .unwrap()
            .split("::")
            .last()
            .unwrap()
            .to_string();

        // process generics if they exist
        if let Some(generics) = split.next() {
            if !generics.ends_with('>') {
                panic!("should end with closing carrot")
            }

            let generics = &generics[0..generics.len() - 1];
            short_name.push('<');
            short_name.push_str(
                &generics
                    .split(',')
                    .map(|generic| Self::get_short_name(generic.trim()))
                    .collect::<Vec<String>>()
                    .join(", "),
            );
            short_name.push('>');
        }
        short_name
    }

    pub fn deserialize<'de, D>(
        &self,
        deserializer: D,
        registry: &PropertyTypeRegistry,
    ) -> Result<Box<dyn Property>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut erased = erased_serde::Deserializer::erase(deserializer);
        (self.deserialize_fn)(&mut erased, registry)
            .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)
    }
}
