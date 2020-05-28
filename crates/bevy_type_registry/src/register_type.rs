
use bevy_property::{Property, Properties, DeserializeProperty};
use legion::storage::Component;
use bevy_app::{FromResources, AppBuilder};
use crate::TypeRegistry;

pub trait RegisterType {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Properties + DeserializeProperty + Component + FromResources;
    fn register_property_type<T>(&mut self) -> &mut Self
    where
        T: Property + DeserializeProperty;
}

impl RegisterType for AppBuilder {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Properties + DeserializeProperty + Component + FromResources,
    {
        {
            let type_registry = self
                .resources()
                .get_mut::<TypeRegistry>()
                .unwrap();
            type_registry.component.write().unwrap().register::<T>();
            type_registry.property.write().unwrap().register::<T>();
        }
        self
    }

    fn register_property_type<T>(&mut self) -> &mut Self
    where
        T: Property + DeserializeProperty {
        {
            let type_registry = self
                .resources()
                .get_mut::<TypeRegistry>()
                .unwrap();
            type_registry.property.write().unwrap().register::<T>();
        }
        self
    }
}
