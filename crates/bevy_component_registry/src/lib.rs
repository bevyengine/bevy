mod component_registry;
mod register_component;

pub use component_registry::*;
pub use register_component::*;

use bevy_app::{AppBuilder, AppPlugin};

#[derive(Default)]
pub struct ComponentRegistryPlugin;

impl AppPlugin for ComponentRegistryPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ComponentRegistryContext>()
            .init_resource::<PropertyTypeRegistryContext>();
    }
}
