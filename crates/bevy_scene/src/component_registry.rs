use bevy_app::AppBuilder;
use legion::{
    prelude::{Entity, World},
    storage::{ArchetypeDescription, ComponentResourceSet, ComponentTypeId},
};
use serde::{de::DeserializeSeed, ser::Serialize, Deserialize};
use std::{collections::HashMap, marker::PhantomData, ptr::NonNull, sync::{RwLock, Arc}};
use crate::world::ComponentSeqDeserializer;

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
        T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>,
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
    pub comp_serialize_fn: fn(&ComponentResourceSet, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    pub individual_comp_serialize_fn:
        fn(&ComponentResourceSet, usize, &mut dyn FnMut(&dyn erased_serde::Serialize)),
    pub comp_deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        get_next_storage_fn: &mut dyn FnMut() -> Option<(NonNull<u8>, usize)>,
    ) -> Result<(), erased_serde::Error>,
    pub individual_comp_deserialize_fn: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
        &mut World,
        Entity,
    ) -> Result<(), erased_serde::Error>,
    pub register_comp_fn: fn(&mut ArchetypeDescription),
    pub short_name: &'static str,
}

impl ComponentRegistration {
    pub fn of<T: Clone + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static>() -> Self {
        let ty = ComponentTypeId::of::<T>();
        Self {
            ty,
            comp_serialize_fn: |comp_storage, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { comp_storage.data_slice::<T>() };
                serialize_fn(&*slice);
            },
            individual_comp_serialize_fn: |comp_storage, index: usize, serialize_fn| {
                // it's safe because we know this is the correct type due to lookup
                let slice = unsafe { comp_storage.data_slice::<T>() };
                serialize_fn(&slice[index]);
            },
            comp_deserialize_fn: |deserializer, get_next_storage_fn| {
                let comp_seq_deser = ComponentSeqDeserializer::<T> {
                    get_next_storage_fn,
                    _marker: PhantomData,
                };
                comp_seq_deser.deserialize(deserializer)?;
                Ok(())
            },
            individual_comp_deserialize_fn: |deserializer, world, entity| {
                let component = erased_serde::deserialize::<T>(deserializer)?;
                world.add_component(entity, component).unwrap();
                Ok(())
            },
            register_comp_fn: |desc| {
                desc.register_component::<T>();
            },
            short_name: ty.0.split("::").last().unwrap(),
        }
    }
}

pub trait RegisterComponent {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>;
}

impl RegisterComponent for AppBuilder {
    fn register_component<T>(&mut self) -> &mut Self
    where
        T: Clone + Send + Sync + 'static + Serialize + for<'de> Deserialize<'de>,
    {
        {
            let registry_context = self.resources().get_mut::<ComponentRegistryContext>().unwrap();
            registry_context.value.write().unwrap().register::<T>();
        }
        self
    }
}
