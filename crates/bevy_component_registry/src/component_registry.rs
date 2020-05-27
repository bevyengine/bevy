use bevy_property::{Properties, Property, PropertyTypeRegistry};
use legion::{
    prelude::{Entity, World, Resources},
    storage::{Component, ComponentResourceSet, ComponentTypeId},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use bevy_app::FromResources;

#[derive(Clone, Default)]
pub struct PropertyTypeRegistryContext {
    pub value: Arc<RwLock<PropertyTypeRegistry>>,
}

#[derive(Default)]
pub struct ComponentRegistryContext {
    pub value: Arc<RwLock<ComponentRegistry>>,
}

#[derive(Default)]
pub struct ComponentRegistry {
    pub registrations: HashMap<ComponentTypeId, ComponentRegistration>,
    pub short_names: HashMap<String, ComponentTypeId>,
    pub full_names: HashMap<String, ComponentTypeId>,
}

impl ComponentRegistry {
    pub fn register<T>(&mut self)
    where
        T: Properties + Component + FromResources,
    {
        let registration = ComponentRegistration::of::<T>();
        self.short_names
            .insert(registration.short_name.to_string(), registration.ty);
        self.full_names
            .insert(registration.ty.0.to_string(), registration.ty);
        self.registrations.insert(registration.ty, registration);
    }

    pub fn get(&self, type_id: &ComponentTypeId) -> Option<&ComponentRegistration> {
        self.registrations.get(type_id)
    }

    pub fn get_with_full_name(&self, full_name: &str) -> Option<&ComponentRegistration> {
        self.full_names
            .get(full_name)
            .and_then(|id| self.registrations.get(id))
    }

    pub fn get_with_short_name(&self, short_name: &str) -> Option<&ComponentRegistration> {
        self.short_names
            .get(short_name)
            .and_then(|id| self.registrations.get(id))
    }
}

#[derive(Clone)]
pub struct ComponentRegistration {
    pub ty: ComponentTypeId,
    pub component_add_fn: fn(&mut World, resources: &Resources, Entity, &dyn Property),
    pub component_properties_fn: fn(&ComponentResourceSet, usize) -> &dyn Properties,
    pub short_name: &'static str,
}

impl ComponentRegistration {
    pub fn of<T: Properties + Component + FromResources>() -> Self {
        let ty = ComponentTypeId::of::<T>();
        Self {
            ty,
            component_add_fn: |world: &mut World, resources: &Resources, entity: Entity, property: &dyn Property| {
                let mut component = T::from_resources(resources);
                component.apply(property);
                world.add_component(entity, component).unwrap();
            },
            component_properties_fn: |component_resource_set: &ComponentResourceSet,
                                      index: usize| {
                // the type has been looked up by the caller, so this is safe
                unsafe { &component_resource_set.data_slice::<T>()[index] }
            },
            short_name: ty.0.split("::").last().unwrap(),
        }
    }
}
