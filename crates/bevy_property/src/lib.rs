#![feature(min_specialization)]

mod property;
mod properties;
mod dynamic_properties;

pub use property::*;
pub use properties::*;
pub use dynamic_properties::*;

pub use bevy_property_derive::*; 
pub use serde;