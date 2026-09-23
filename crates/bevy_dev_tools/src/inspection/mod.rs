//! Entity and world inspection tools for Bevy.
//!
//! This module uses a front-end/backend architecture,
//! dividing its responsibilities between extracting data about the world state,
//! and presenting that data to the user in a number of convenient, often interactive ways.

pub mod component_inspection;
pub mod entity_inspection;
pub mod extension_methods;
pub mod label_resolution;
pub mod reflection_tools;
pub mod resource_inspection;
pub mod world_summary;
