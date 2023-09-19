use std::any::TypeId;

use bevy_reflect::{Reflect, ReflectFromPtr, TraitTypeData};

use crate::component::ComponentId;
use crate::entity::Entity;
use crate::prelude::{AppTypeRegistry, Mut, World};

impl World {
    /// Returns the [`TypeId`] of the underlying component type. Returns None if the component does not correspond to a Rust type.
    pub fn component_type_id(&self, component_id: ComponentId) -> Option<TypeId> {
        self.components()
            .get_info(component_id)
            .and_then(|n| n.type_id())
    }

    /// Retrieves an immutable `dyn T` reference to the given entity's Component of the given [`ComponentId`]
    pub fn get_dyn_by_id<T: TraitTypeData>(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&T::Dyn> {
        let type_id = self.component_type_id(component_id)?;

        let dyn_obj = self.get_dyn_reflect_by_id(entity, component_id)?;

        let type_registry = self.resource::<AppTypeRegistry>();
        let type_registry = type_registry.read();
        let type_data = type_registry.get_type_data::<T>(type_id)?;
        type_data.get(dyn_obj)
    }

    /// Retrieves an mutable `dyn T` reference to the given entity's Component of the given [`ComponentId`]
    pub fn get_dyn_mut_by_id<T: TraitTypeData>(
        &mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&mut T::Dyn> {
        let type_id = self.component_type_id(component_id)?;

        let type_data: Box<T> = ({
            let type_registry = self.get_resource::<AppTypeRegistry>()?;
            let type_registry = type_registry.read();
            type_registry.get_type_data::<T>(type_id)?.clone_type_data()
        })
        .downcast::<T>()
        .ok()?;
        let dyn_obj = self.get_dyn_reflect_mut_by_id(entity, component_id)?;
        type_data.get_mut(dyn_obj.value)
    }

    /// Retrieves an immutable `dyn Reflect` reference to the given entity's Component of the given [`ComponentId`]
    pub fn get_dyn_reflect_by_id(
        &self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<&dyn Reflect> {
        let type_id = self.component_type_id(component_id)?;
        let component_ptr = self.get_by_id(entity, component_id)?;

        let type_registry = self.get_resource::<AppTypeRegistry>()?;
        let type_registry = type_registry.read();
        let from_ptr = type_registry.get_type_data::<ReflectFromPtr>(type_id)?;
        // SAFETY: type_id is correct
        Some(unsafe { from_ptr.as_reflect(component_ptr) })
    }

    /// Retrieves an mutable `dyn Reflect` reference to the given entity's Component of the given [`ComponentId`]
    pub fn get_dyn_reflect_mut_by_id<'a>(
        &'a mut self,
        entity: Entity,
        component_id: ComponentId,
    ) -> Option<Mut<'a, dyn Reflect>> {
        let type_id = self.component_type_id(component_id)?;
        let reflect_component_ptr = {
            let type_registry = self.get_resource::<AppTypeRegistry>()?;
            let type_registry = type_registry.read();
            type_registry
                .get_type_data::<ReflectFromPtr>(type_id)?
                .clone()
        };

        let mut_ptr = self.get_mut_by_id(entity, component_id)?;
        Some(mut_ptr.map_unchanged(|p| {
            // SAFETY: type_id is correct
            unsafe { reflect_component_ptr.as_reflect_mut(p) }
        }))
    }
}

#[cfg(test)]
mod tests {
    use bevy_reflect::{reflect_trait, Reflect};
    use std::ops::DerefMut;

    use crate::prelude::AppTypeRegistry;
    use crate::{self as bevy_ecs, component::Component, world::World};

    #[reflect_trait]
    trait DoThing: Reflect {
        fn do_thing(&self) -> String;
        fn mut_do_thing(&mut self);
    }

    #[reflect_trait]
    trait OtherTrait: Reflect {
        fn other_do_thing(&self) -> String;
    }

    #[derive(Component, Reflect)]
    #[reflect(DoThing, OtherTrait)]
    struct ComponentA(String);

    impl DoThing for ComponentA {
        fn do_thing(&self) -> String {
            format!("ComponentA {} do_thing!", self.0)
        }

        fn mut_do_thing(&mut self) {
            self.0 = "value changed".to_string();
        }
    }

    impl OtherTrait for ComponentA {
        fn other_do_thing(&self) -> String {
            format!("ComponentA {} other do_thing!", self.0)
        }
    }

    #[test]
    fn dyn_reflect() {
        let mut world = World::new();
        let world = &mut world;

        let type_registry = AppTypeRegistry::default();
        {
            let mut registry = type_registry.write();
            registry.register::<ComponentA>();
            registry.register_type_data::<ComponentA, ReflectDoThing>();
        }
        world.insert_resource(type_registry);

        let entity = world.spawn(ComponentA("value".to_string())).id();

        let component_id = world.component_id::<ComponentA>().unwrap();
        let component_type_id = world.component_type_id(component_id).unwrap();

        {
            let do_thing = world.get_dyn_reflect_by_id(entity, component_id);
            assert!(do_thing.is_some());
            let do_thing = do_thing.unwrap();
            assert_eq!(do_thing.type_id(), component_type_id);
        }

        {
            let do_thing = world.get_dyn_reflect_mut_by_id(entity, component_id);
            assert!(do_thing.is_some());
            let mut do_thing = do_thing.unwrap();
            let do_thing = do_thing.deref_mut();
            assert_eq!(do_thing.type_id(), component_type_id);
        }

        {
            let do_thing = world.get_dyn_mut_by_id::<ReflectDoThing>(entity, component_id);
            assert!(do_thing.is_some());
            let mut do_thing = do_thing.unwrap();
            let do_thing = do_thing.deref_mut();
            do_thing.mut_do_thing();
            assert_eq!(do_thing.type_id(), component_type_id);
        }

        {
            let do_thing = world.get_dyn_by_id::<ReflectDoThing>(entity, component_id);
            assert!(do_thing.is_some());
            let do_thing = do_thing.unwrap();
            do_thing.do_thing();
            assert_eq!(do_thing.as_reflect().type_id(), component_type_id);
        }

        {
            let other_trait = world.get_dyn_mut_by_id::<ReflectOtherTrait>(entity, component_id);
            assert!(other_trait.is_some());
            let other_trait = other_trait.unwrap();
            assert_eq!(other_trait.as_reflect().type_id(), component_type_id);
        }
        {
            let other_trait = world.get_dyn_by_id::<ReflectOtherTrait>(entity, component_id);
            assert!(other_trait.is_some());
            let other_trait = other_trait.unwrap();
            other_trait.other_do_thing();
            assert_eq!(other_trait.as_reflect().type_id(), component_type_id);
        }
    }
}
