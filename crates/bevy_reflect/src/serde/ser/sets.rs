use crate::{serde::TypedReflectSerializer, set::SetLike, TypeRegistry};
use serde::{ser::SerializeSeq, Serialize};

use super::ReflectSerializerProcessor;

/// A serializer for [`Set`] values.
pub(super) struct SetSerializer<'a, SL: SetLike + ?Sized, P> {
    pub set: &'a SL,
    pub registry: &'a TypeRegistry,
    pub processor: Option<&'a P>,
}

impl<SL: SetLike + ?Sized, P: ReflectSerializerProcessor> Serialize for SetSerializer<'_, SL, P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_seq(Some(self.set.len()))?;
        for value in self.set.iter() {
            state.serialize_element(&TypedReflectSerializer::new_internal(
                value,
                self.registry,
                self.processor,
            ))?;
        }
        state.end()
    }
}
