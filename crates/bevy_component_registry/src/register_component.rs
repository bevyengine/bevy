
use bevy_property::{Property, Properties};
use legion::storage::Component;
use serde::Deserialize;
use crate::{PropertyTypeRegistryContext, ComponentRegistryContext};
use bevy_app::AppBuilder;

pub trait RegisterComponent {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Properties + Component + Default;
    fn register_property_type<T>(&mut self) -> &mut Self
    where
        T: Property + for<'de> Deserialize<'de>;
}

impl RegisterComponent for AppBuilder {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Properties + Component + Default,
    {
        {
            let registry_context = self
                .resources()
                .get_mut::<ComponentRegistryContext>()
                .unwrap();
            registry_context.value.write().unwrap().register::<T>();
        }
        self
    }

    fn register_property_type<T>(&mut self) -> &mut Self
    where
        T: Property + for<'de> Deserialize<'de>,
    {
        {
            let registry_context = self
                .resources()
                .get_mut::<PropertyTypeRegistryContext>()
                .unwrap();
            registry_context.value.write().unwrap().register::<T>();
        }
        self
    }
}
