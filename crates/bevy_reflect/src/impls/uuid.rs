use bevy_utils::Uuid;
use crate::{ReflectSerialize, ReflectDeserialize};
use bevy_reflect_derive::{impl_reflect_value, impl_from_reflect_value};

impl_reflect_value!(Uuid(Serialize, Deserialize));
impl_from_reflect_value!(Uuid);
