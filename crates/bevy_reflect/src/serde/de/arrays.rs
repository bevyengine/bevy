use crate::{
    serde::{de::registration_utils::try_get_registration, TypedReflectDeserializer},
    ArrayInfo, DynamicArray, TypeRegistry,
};
use core::{fmt, fmt::Formatter};
use serde::de::{Error, SeqAccess, Visitor};

/// A [`Visitor`] for deserializing [`Array`] values.
///
/// [`Array`]: crate::Array
pub(super) struct ArrayVisitor<'a> {
    array_info: &'static ArrayInfo,
    registry: &'a TypeRegistry,
}

impl<'a> ArrayVisitor<'a> {
    pub fn new(array_info: &'static ArrayInfo, registry: &'a TypeRegistry) -> Self {
        Self {
            array_info,
            registry,
        }
    }
}

impl<'a, 'de> Visitor<'de> for ArrayVisitor<'a> {
    type Value = DynamicArray;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("reflected array value")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut vec = Vec::with_capacity(seq.size_hint().unwrap_or_default());
        let registration = try_get_registration(self.array_info.item_ty(), self.registry)?;
        while let Some(value) = seq.next_element_seed(TypedReflectDeserializer::new_internal(
            registration,
            self.registry,
        ))? {
            vec.push(value);
        }

        if vec.len() != self.array_info.capacity() {
            return Err(Error::invalid_length(
                vec.len(),
                &self.array_info.capacity().to_string().as_str(),
            ));
        }

        Ok(DynamicArray::new(vec.into_boxed_slice()))
    }
}
