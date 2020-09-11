use crate::{
    property_serde::DynamicPropertiesDeserializer, DynamicProperties, PropertyTypeRegistry,
};
use ron::de::Deserializer;
use serde::de::DeserializeSeed;

pub fn deserialize_dynamic_properties(
    ron_string: &str,
    property_type_registry: &PropertyTypeRegistry,
) -> Result<DynamicProperties, ron::Error> {
    let mut deserializer = Deserializer::from_str(&ron_string).unwrap();
    let dynamic_properties_deserializer =
        DynamicPropertiesDeserializer::new(&property_type_registry);
    dynamic_properties_deserializer.deserialize(&mut deserializer)
}
