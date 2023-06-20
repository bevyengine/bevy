use crate as bevy_reflect;

use crate::{ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::{impl_from_reflect_value, impl_reflect_value};
use bevy_utils::Uuid;

impl_reflect_value!(::bevy_utils::Uuid(Serialize, Deserialize));
impl_from_reflect_value!(Uuid);
