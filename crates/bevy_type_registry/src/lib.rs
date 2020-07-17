mod register_type;
mod type_registry;

pub use register_type::*;
pub use type_registry::*;

use bevy_app::prelude::*;
use bevy_property::DynamicProperties;

#[derive(Default)]
pub struct TypeRegistryPlugin;

impl AppPlugin for TypeRegistryPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<TypeRegistry>()
            .register_property_type::<DynamicProperties>();
    }
}
