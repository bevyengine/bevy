use crate::{
    property_serde::DynamicPropertiesDeserializer, DynamicProperties, PropertyTypeRegistry,
};
use ron::de::Deserializer;
use serde::de::DeserializeSeed;
use std::{cell::RefCell, rc::Rc};

pub fn deserialize_dynamic_properties(
    ron_string: &str,
    property_type_registry: &PropertyTypeRegistry,
) -> Result<DynamicProperties, ron::Error> {
    let mut deserializer = Deserializer::from_str(&ron_string).unwrap();
    let last_type_name = Rc::new(RefCell::new(None));
    let mut callback = |ident: &Option<&[u8]>| {
        let mut last_type_name = last_type_name.borrow_mut();
        *last_type_name = ident.map(|i| String::from_utf8(i.to_vec()).unwrap());
    };
    deserializer.set_callback(&mut callback);
    let dynamic_properties_deserializer = DynamicPropertiesDeserializer {
        current_type_name: last_type_name.clone(),
        registry: &property_type_registry,
    };
    dynamic_properties_deserializer.deserialize(&mut deserializer)
}
