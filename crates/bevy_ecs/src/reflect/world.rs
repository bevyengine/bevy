use bevy_reflect::{Reflect, ReflectFnsTypeData, ReflectFromPtr};

use crate::component::ComponentId;
use crate::entity::Entity;
use crate::prelude::{AppTypeRegistry, World};

/// An extension trait for World for reflection related functions
pub trait WorldExt {
    /// Retrieves an immutable `dyn T` reference to the given entity's Component of the given ComponentId
    fn get_dyn_by_id<T: ReflectFnsTypeData>(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&T::Dyn>;

    /// Retrieves an mutable `dyn T` reference to the given entity's Component of the given ComponentId
    fn get_dyn_mut_by_id<T: ReflectFnsTypeData>(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&mut T::Dyn>;

    /// Retrieves an immutable `dyn Reflect` reference to the given entity's Component of the given ComponentId
    fn get_dyn_reflect_by_id(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&dyn Reflect>;

    /// Retrieves an mutable `dyn Reflect` reference to the given entity's Component of the given ComponentId
    fn get_dyn_reflect_mut_by_id(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&mut dyn Reflect>;
}

impl WorldExt for World {
    fn get_dyn_by_id<T: ReflectFnsTypeData>(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&T::Dyn> {
        let Some(type_id) = self
            .components()
            .get_info(component_id)
            .and_then(|n| n.type_id())
        else {
            return None;
        };

        let Some(dyn_obj) = self.get_dyn_reflect_by_id(entity, component_id) else {
            return None;
        };

        let type_registry = self.resource::<AppTypeRegistry>();
        let type_registry = type_registry.read();
        type_registry
            .get_type_data::<T>(type_id)
            .and_then(|n| n.get(dyn_obj))
    }

    fn get_dyn_mut_by_id<T: ReflectFnsTypeData>(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&mut T::Dyn> {
        let Some(type_id) = self
            .components()
            .get_info(component_id)
            .and_then(|n| n.type_id())
        else {
            return None;
        };

        let type_registry = self.resource::<AppTypeRegistry>().clone();
        let type_registry = type_registry.read();
        let Some(type_data) = type_registry.get_type_data::<T>(type_id) else {
            return None;
        };
        self.get_dyn_reflect_mut_by_id(entity, component_id)
            .and_then(|dyn_obj| type_data.get_mut(dyn_obj))
    }

    fn get_dyn_reflect_by_id(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&dyn Reflect> {
        let Some(type_id) = self
            .components()
            .get_info(component_id)
            .and_then(|n| n.type_id())
        else {
            return None;
        };
        let Some(component_ptr) = self.get_by_id(entity, component_id) else {
            return None;
        };

        let type_registry = self.resource::<AppTypeRegistry>();
        let type_registry = type_registry.read();
        type_registry
            .get_type_data::<ReflectFromPtr>(type_id)
            .map(|n| unsafe { n.as_reflect(component_ptr) })
    }

    fn get_dyn_reflect_mut_by_id(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&mut dyn Reflect> {
        let Some(type_id) = self
            .components()
            .get_info(component_id)
            .and_then(|n| n.type_id())
        else {
            return None;
        };

        let type_registry = self.resource::<AppTypeRegistry>().clone();
        let type_registry = type_registry.read();
        let Some(reflect_component_ptr) = type_registry.get_type_data::<ReflectFromPtr>(type_id)
        else {
            return None;
        };

        self.get_mut_by_id(entity, component_id).map(|n| {
            n.map_unchanged(|p| unsafe { reflect_component_ptr.as_reflect_mut(p) })
                .value
        })
    }
}

#[cfg(test)]
mod tests {
    use bevy_reflect::{reflect_trait, Reflect};

    use crate::prelude::AppTypeRegistry;
    use crate::reflect::WorldExt;
    use crate::{self as bevy_ecs, component::Component, world::World};

    #[reflect_trait]
    trait DoThing {
        fn do_thing(&self) -> String;
        fn mut_do_thing(&mut self);
    }

    #[derive(Component, Reflect)]
    #[reflect(DoThing)]
    struct ComponentA(String);

    impl DoThing for ComponentA {
        fn do_thing(&self) -> String {
            format!("ComponentA {} do_thing!", self.0)
        }

        fn mut_do_thing(&mut self) {
            self.0 = "value changed".to_string()
        }
    }

    #[test]
    fn dyn_reflect() {
        let mut world = World::new();

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<ComponentA>();
            registry.register_type_data::<ComponentA, ReflectDoThing>();
        }
        world.insert_resource(type_registry);

        let entity = world.spawn(ComponentA("value".to_string())).id();

        let component_id = world.component_id::<ComponentA>().unwrap();

        let _do_thing = world
            .get_dyn_reflect_mut_by_id(entity, component_id)
            .unwrap();
        let _do_thing = world.get_dyn_reflect_by_id(entity, component_id).unwrap();

        let do_thing = world
            .get_dyn_mut_by_id::<ReflectDoThing>(entity, component_id)
            .unwrap();
        do_thing.mut_do_thing();

        let do_thing = world
            .get_dyn_by_id::<ReflectDoThing>(entity, component_id)
            .unwrap();
        do_thing.do_thing();
    }
}
