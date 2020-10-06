use crate::{DeserializeProperty, Property};
use bevy_utils::{HashMap, HashSet};
use std::{any::TypeId, fmt};

#[derive(Debug, Default)]
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

impl fmt::Debug for PropertyTypeRegistration {
    fn fmt<'a>(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PropertyTypeRegistration")
            .field("ty", &self.ty)
            .field(
                "deserialize_fn",
                &(self.deserialize_fn
                    as fn(
                        deserializer: &'a mut dyn erased_serde::Deserializer<'a>,
                        property_type_registry: &'a PropertyTypeRegistry,
                    ) -> Result<Box<dyn Property>, erased_serde::Error>),
            )
            .field("short_name", &self.short_name)
            .field("name", &self.name)
            .finish()
    }
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
        let mut short_name = String::new();

        {
            // A typename may be a composition of several other type names (e.g. generic parameters)
            // separated by the characters that we try to find below.
            // Then, each individual typename is shortened to its last path component.
            //
            // Note: Instead of `find`, `split_inclusive` would be nice but it's still unstable...
            let mut remainder = full_name;
            while let Some(index) = remainder.find(&['<', '>', '(', ')', '[', ']', ',', ';'][..]) {
                let (path, new_remainder) = remainder.split_at(index);
                // Push the shortened path in front of the found character
                short_name.push_str(path.rsplit(':').next().unwrap());
                // Push the character that was found
                let character = new_remainder.chars().next().unwrap();
                short_name.push(character);
                // Advance the remainder
                if character == ',' || character == ';' {
                    // A comma or semicolon is always followed by a space
                    short_name.push(' ');
                    remainder = &new_remainder[2..];
                } else {
                    remainder = &new_remainder[1..];
                }
            }

            // The remainder will only be non-empty if there were no matches at all
            if !remainder.is_empty() {
                // Then, the full typename is a path that has to be shortened
                short_name.push_str(remainder.rsplit(':').next().unwrap());
            }
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

#[cfg(test)]
mod test {
    use crate::PropertyTypeRegistration;
    use std::collections::HashMap;

    #[test]
    fn test_get_short_name() {
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<f64>()),
            "f64"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<String>()),
            "String"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<(u32, f64)>()),
            "(u32, f64)"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<(String, String)>()),
            "(String, String)"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<[f64]>()),
            "[f64]"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<[String]>()),
            "[String]"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<[f64; 16]>()),
            "[f64; 16]"
        );
        assert_eq!(
            PropertyTypeRegistration::get_short_name(std::any::type_name::<[String; 16]>()),
            "[String; 16]"
        );
    }

    #[test]
    fn test_property_type_registration() {
        assert_eq!(
            PropertyTypeRegistration::of::<Option<f64>>().short_name,
            "Option<f64>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<HashMap<u32, String>>().short_name,
            "HashMap<u32, String>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<Option<HashMap<u32, String>>>().short_name,
            "Option<HashMap<u32, String>>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<Option<HashMap<u32, Option<String>>>>().short_name,
            "Option<HashMap<u32, Option<String>>>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<Option<HashMap<String, Option<String>>>>().short_name,
            "Option<HashMap<String, Option<String>>>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<Option<HashMap<Option<String>, Option<String>>>>()
                .short_name,
            "Option<HashMap<Option<String>, Option<String>>>"
        );
        assert_eq!(
            PropertyTypeRegistration::of::<Option<HashMap<Option<String>, (String, Option<String>)>>>()
                .short_name,
            "Option<HashMap<Option<String>, (String, Option<String>)>>"
        );
    }
}
