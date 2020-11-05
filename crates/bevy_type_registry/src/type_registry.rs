use bevy_ecs::{
    Archetype, Component, Entity, EntityMap, FromResources, MapEntities, MapEntitiesError,
    Resources, World,
};
use bevy_property::{
    DeserializeProperty, Properties, Property, PropertyTypeRegistration, PropertyTypeRegistry,
};
use bevy_utils::{HashMap, HashSet};
use parking_lot::RwLock;
use std::{any::TypeId, marker::PhantomData, sync::Arc};

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
        self.add_registration(ComponentRegistration::of::<T>());
    }

    pub fn add_registration(&mut self, registration: ComponentRegistration) {
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

    pub fn iter(&self) -> impl Iterator<Item = &ComponentRegistration> {
        self.registrations.values()
    }
}

#[derive(Clone)]
pub struct ComponentRegistration {
    pub ty: TypeId,
    pub short_name: String,
    pub long_name: &'static str,
    component_add_fn: fn(&mut World, resources: &Resources, Entity, &dyn Property),
    component_apply_fn: fn(&mut World, Entity, &dyn Property),
    component_properties_fn: fn(&Archetype, usize) -> &dyn Properties,
    component_copy_fn: fn(&World, &mut World, &Resources, Entity, Entity),
    copy_to_scene_fn: fn(&World, &mut World, &Resources, Entity, Entity),
    copy_from_scene_fn: fn(&World, &mut World, &Resources, Entity, Entity),
    map_entities_fn: fn(&mut World, &EntityMap) -> Result<(), MapEntitiesError>,
}

struct ComponentRegistrationDefaults;

impl ComponentRegistrationDefaults {
    pub fn component_add<T: Component + Properties + FromResources>(
        world: &mut World,
        resources: &Resources,
        entity: Entity,
        property: &dyn Property,
    ) {
        let mut component = T::from_resources(resources);
        component.apply(property);
        world.insert_one(entity, component).unwrap();
    }

    fn component_apply<T: Component + Properties>(
        world: &mut World,
        entity: Entity,
        property: &dyn Property,
    ) {
        let mut component = world.get_mut::<T>(entity).unwrap();
        component.apply(property);
    }

    fn component_copy<T: Component + Properties + FromResources>(
        source_world: &World,
        destination_world: &mut World,
        resources: &Resources,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        let source_component = source_world.get::<T>(source_entity).unwrap();
        let mut destination_component = T::from_resources(resources);
        destination_component.apply(source_component);
        destination_world
            .insert_one(destination_entity, destination_component)
            .unwrap();
    }

    fn component_properties<T: Component + Properties>(
        archetype: &Archetype,
        index: usize,
    ) -> &dyn Properties {
        // the type has been looked up by the caller, so this is safe
        unsafe {
            let ptr = archetype.get::<T>().unwrap().as_ptr().add(index);
            ptr.as_ref().unwrap()
        }
    }

    fn map_entities(_world: &mut World, _entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        Ok(())
    }
}

impl ComponentRegistration {
    pub fn build<T>() -> ComponentRegistrationBuilder<T>
    where
        T: Properties + DeserializeProperty + Component + FromResources,
    {
        ComponentRegistrationBuilder {
            registration: ComponentRegistration::of::<T>(),
            marker: PhantomData::default(),
        }
    }

    pub fn of<T: Properties + Component + FromResources>() -> Self {
        let ty = TypeId::of::<T>();
        Self {
            ty,
            component_add_fn: ComponentRegistrationDefaults::component_add::<T>,
            component_apply_fn: ComponentRegistrationDefaults::component_apply::<T>,
            component_copy_fn: ComponentRegistrationDefaults::component_copy::<T>,
            component_properties_fn: ComponentRegistrationDefaults::component_properties::<T>,
            copy_from_scene_fn: ComponentRegistrationDefaults::component_copy::<T>,
            copy_to_scene_fn: ComponentRegistrationDefaults::component_copy::<T>,
            map_entities_fn: ComponentRegistrationDefaults::map_entities,
            short_name: PropertyTypeRegistration::get_short_name(std::any::type_name::<T>()),
            long_name: std::any::type_name::<T>(),
        }
    }

    pub fn add_property_to_entity(
        &self,
        world: &mut World,
        resources: &Resources,
        entity: Entity,
        property: &dyn Property,
    ) {
        (self.component_add_fn)(world, resources, entity, property);
    }

    pub fn apply_property_to_entity(
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

    pub fn component_copy(
        &self,
        source_world: &World,
        destination_world: &mut World,
        resources: &Resources,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.component_copy_fn)(
            source_world,
            destination_world,
            resources,
            source_entity,
            destination_entity,
        );
    }

    pub fn copy_from_scene(
        &self,
        scene_world: &World,
        destination_world: &mut World,
        resources: &Resources,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.component_copy_fn)(
            scene_world,
            destination_world,
            resources,
            source_entity,
            destination_entity,
        );
    }

    pub fn copy_to_scene(
        &self,
        source_world: &World,
        scene_world: &mut World,
        resources: &Resources,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.copy_to_scene_fn)(
            source_world,
            scene_world,
            resources,
            source_entity,
            destination_entity,
        );
    }

    pub fn map_entities(
        &self,
        world: &mut World,
        entity_map: &EntityMap,
    ) -> Result<(), MapEntitiesError> {
        (self.map_entities_fn)(world, entity_map)
    }
}

pub struct ComponentRegistrationBuilder<T> {
    registration: ComponentRegistration,
    marker: PhantomData<T>,
}

impl<T> ComponentRegistrationBuilder<T>
where
    T: Properties + DeserializeProperty + Component + FromResources,
{
    pub fn map_entities(mut self) -> Self
    where
        T: MapEntities,
    {
        self.registration.map_entities_fn = |world: &mut World, entity_map: &EntityMap| {
            // TODO: add UntrackedMut<T> pointer that returns &mut T. This will avoid setting the "mutated" state
            for mut component in &mut world.query_mut::<&mut T>() {
                component.map_entities(entity_map)?;
            }

            Ok(())
        };
        self
    }

    pub fn into_scene_component<C: Component>(mut self) -> Self
    where
        T: IntoComponent<C>,
    {
        self.registration.copy_to_scene_fn =
            |source_world: &World,
             scene_world: &mut World,
             resources: &Resources,
             source_entity: Entity,
             scene_entity: Entity| {
                let source_component = source_world.get::<T>(source_entity).unwrap();
                let scene_component = source_component.into_component(resources);
                scene_world
                    .insert_one(scene_entity, scene_component)
                    .unwrap();
            };

        self
    }

    pub fn into_runtime_component<C: Component>(mut self) -> Self
    where
        T: IntoComponent<C>,
    {
        self.registration.copy_from_scene_fn =
            |scene_world: &World,
             destination_world: &mut World,
             resources: &Resources,
             scene_entity: Entity,
             destination_entity: Entity| {
                let scene_component = scene_world.get::<T>(scene_entity).unwrap();
                let destination_component = scene_component.into_component(resources);
                destination_world
                    .insert_one(destination_entity, destination_component)
                    .unwrap();
            };

        self
    }

    pub fn finish(self) -> ComponentRegistration {
        self.registration
    }
}

pub trait IntoComponent<ToComponent: Component> {
    fn into_component(&self, resources: &Resources) -> ToComponent;
}
