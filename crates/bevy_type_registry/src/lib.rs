mod register_type;
mod type_registry;
mod type_uuid;

pub use register_type::*;
pub use type_registry::*;
pub use type_uuid::*;
pub use uuid::Uuid;

use bevy_app::prelude::*;
use bevy_property::DynamicProperties;

#[derive(Default)]
pub struct TypeRegistryPlugin;

impl Plugin for TypeRegistryPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<TypeRegistry>()
            .register_property::<DynamicProperties>();
    }
}
