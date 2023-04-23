use crate::impl_reflect_value;
use crate::prelude::ReflectDefault;
use crate::{self as bevy_reflect, ReflectDeserialize, ReflectSerialize};
use bevy_reflect_derive::impl_from_reflect_value;
use uuid::Uuid;

impl_reflect_value!(Uuid(
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));

impl_from_reflect_value!(Uuid);
