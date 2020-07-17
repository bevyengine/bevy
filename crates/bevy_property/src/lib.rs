pub mod impl_property;
pub mod property_serde;
pub mod ron;

mod dynamic_properties;
mod properties;
mod property;
mod type_registry;

pub use dynamic_properties::*;
pub use properties::*;
pub use property::*;
pub use type_registry::*;

pub use bevy_property_derive::*;
pub use erased_serde;
pub use serde;

pub mod prelude {
    pub use crate::{DynamicProperties, Properties, PropertiesVal, Property, PropertyVal};
}
