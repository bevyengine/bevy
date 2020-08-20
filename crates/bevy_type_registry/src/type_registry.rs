use bevy_ecs::{Archetype, Component, Entity, FromResources, Resources, World};
use bevy_property::{Properties, Property, PropertyTypeRegistration, PropertyTypeRegistry};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

#[derive(Clone, Default)]
pub struct TypeRegistry {
    pub property: Arc<RwLock<PropertyTypeRegistry>>,
    pub component: Arc<RwLock<ComponentRegistry>>,
}

#[derive(Default)]
pub struct ComponentRegistry {
    pub registrations: HashMap<TypeId, ComponentRegistration>,
    pub short_names: HashMap<String, TypeId>,
    pub full_names: HashMap<String, TypeId>,
    pub ambigous_names: HashSet<String>,
}

impl ComponentRegistry {
    pub fn register<T>(&mut self)
    where
        T: Properties + Component + FromResources,
    {
        let registration = ComponentRegistration::of::<T>();
        let short_name = registration.short_name.to_string();
        self.full_names
            .insert(registration.long_name.to_string(), registration.ty);
        if self.short_names.contains_key(&short_name) || self.ambigous_names.contains(&short_name) {
            // name is ambiguous. fall back to long names for all ambiguous types
            self.short_names.remove(&short_name);
            self.ambigous_names.insert(short_name);
        } else {
            self.short_names.insert(short_name, registration.ty);
        }
        self.registrations.insert(registration.ty, registration);
    }

    pub fn get(&self, type_id: &TypeId) -> Option<&ComponentRegistration> {
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

    pub fn get_with_name(&self, type_name: &str) -> Option<&ComponentRegistration> {
        let mut registration = self.get_with_short_name(type_name);
        if registration.is_none() {
            registration = self.get_with_full_name(type_name);
            if registration.is_none() && self.ambigous_names.contains(type_name) {
                panic!("Type name is ambiguous: {}", type_name);
            }
        }
        registration
    }
}

#[derive(Clone)]
pub struct ComponentRegistration {
    pub ty: TypeId,
    component_add_fn: fn(&mut World, resources: &Resources, Entity, &dyn Property),
    component_apply_fn: fn(&mut World, Entity, &dyn Property),
    component_properties_fn: fn(&Archetype, usize) -> &dyn Properties,
    pub short_name: String,
    pub long_name: &'static str,
}

impl ComponentRegistration {
    pub fn of<T: Properties + Component + FromResources>() -> Self {
        let ty = TypeId::of::<T>();
        Self {
            ty,
            component_add_fn: |world: &mut World,
                               resources: &Resources,
                               entity: Entity,
                               property: &dyn Property| {
                let mut component = T::from_resources(resources);
                component.apply(property);
                world.insert_one(entity, component).unwrap();
            },
            component_apply_fn: |world: &mut World, entity: Entity, property: &dyn Property| {
                let mut component = world.get_mut::<T>(entity).unwrap();
                component.apply(property);
            },
            component_properties_fn: |archetype: &Archetype, index: usize| {
                // the type has been looked up by the caller, so this is safe
                unsafe {
                    let ptr = archetype.get::<T>().unwrap().as_ptr().add(index);
                    ptr.as_ref().unwrap()
                }
            },
            short_name: PropertyTypeRegistration::get_short_name(std::any::type_name::<T>()),
            long_name: std::any::type_name::<T>(),
        }
    }

    pub fn add_component_to_entity(
        &self,
        world: &mut World,
        resources: &Resources,
        entity: Entity,
        property: &dyn Property,
    ) {
        (self.component_add_fn)(world, resources, entity, property);
    }

    pub fn apply_component_to_entity(
        &self,
        world: &mut World,
        entity: Entity,
        property: &dyn Property,
    ) {
        (self.component_apply_fn)(world, entity, property);
    }

    pub fn get_component_properties<'a>(
        &self,
        archetype: &'a Archetype,
        entity_index: usize,
    ) -> &'a dyn Properties {
        (self.component_properties_fn)(archetype, entity_index)
    }
}
