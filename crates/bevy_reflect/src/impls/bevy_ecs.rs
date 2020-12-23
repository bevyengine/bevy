use crate::{FromType, Reflect};
use bevy_ecs::{
    Archetype, Component, Entity, EntityMap, FromResources, MapEntities, MapEntitiesError,
    Resources, World,
};
use std::marker::PhantomData;

#[derive(Clone)]
pub struct ReflectComponent {
    add_component: fn(&mut World, resources: &Resources, Entity, &dyn Reflect),
    apply_component: fn(&mut World, Entity, &dyn Reflect),
    reflect_component: unsafe fn(&Archetype, usize) -> &dyn Reflect,
    copy_component: fn(&World, &mut World, &Resources, Entity, Entity),
}

impl ReflectComponent {
    pub fn add_component(
        &self,
        world: &mut World,
        resources: &Resources,
        entity: Entity,
        component: &dyn Reflect,
    ) {
        (self.add_component)(world, resources, entity, component);
    }

    pub fn apply_component(&self, world: &mut World, entity: Entity, component: &dyn Reflect) {
        (self.apply_component)(world, entity, component);
    }

    /// # Safety
    /// This does not do bound checks on entity_index. You must make sure entity_index is within bounds before calling.
    pub unsafe fn reflect_component<'a>(
        &self,
        archetype: &'a Archetype,
        entity_index: usize,
    ) -> &'a dyn Reflect {
        (self.reflect_component)(archetype, entity_index)
    }

    pub fn copy_component(
        &self,
        source_world: &World,
        destination_world: &mut World,
        resources: &Resources,
        source_entity: Entity,
        destination_entity: Entity,
    ) {
        (self.copy_component)(
            source_world,
            destination_world,
            resources,
            source_entity,
            destination_entity,
        );
    }
}

impl<C: Component + Reflect + FromResources> FromType<C> for ReflectComponent {
    fn from_type() -> Self {
        ReflectComponent {
            add_component: |world, resources, entity, reflected_component| {
                let mut component = C::from_resources(resources);
                component.apply(reflected_component);
                world.insert_one(entity, component).unwrap();
            },
            apply_component: |world, entity, reflected_component| {
                let mut component = world.get_mut::<C>(entity).unwrap();
                component.apply(reflected_component);
            },
            copy_component: |source_world,
                             destination_world,
                             resources,
                             source_entity,
                             destination_entity| {
                let source_component = source_world.get::<C>(source_entity).unwrap();
                let mut destination_component = C::from_resources(resources);
                destination_component.apply(source_component);
                destination_world
                    .insert_one(destination_entity, destination_component)
                    .unwrap();
            },
            reflect_component: |archetype, index| {
                unsafe {
                    // the type has been looked up by the caller, so this is safe
                    let ptr = archetype.get::<C>().unwrap().as_ptr().add(index);
                    ptr.as_ref().unwrap()
                }
            },
        }
    }
}

#[derive(Clone)]
pub struct SceneComponent<Scene: Component, Runtime: Component> {
    copy_scene_to_runtime: fn(&World, &mut World, &Resources, Entity, Entity),
    marker: PhantomData<(Scene, Runtime)>,
}

impl<Scene: Component + IntoComponent<Runtime>, Runtime: Component> SceneComponent<Scene, Runtime> {
    pub fn copy_scene_to_runtime(
        &self,
        scene_world: &World,
        runtime_world: &mut World,
        resources: &Resources,
        scene_entity: Entity,
        runtime_entity: Entity,
    ) {
        (self.copy_scene_to_runtime)(
            scene_world,
            runtime_world,
            resources,
            scene_entity,
            runtime_entity,
        );
    }
}

impl<Scene: Component + IntoComponent<Runtime>, Runtime: Component> FromType<Scene>
    for SceneComponent<Scene, Runtime>
{
    fn from_type() -> Self {
        SceneComponent {
            copy_scene_to_runtime: |scene_world,
                                    runtime_world,
                                    resources,
                                    scene_entity,
                                    runtime_entity| {
                let scene_component = scene_world.get::<Scene>(scene_entity).unwrap();
                let destination_component = scene_component.into_component(resources);
                runtime_world
                    .insert_one(runtime_entity, destination_component)
                    .unwrap();
            },
            marker: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct RuntimeComponent<Runtime: Component, Scene: Component> {
    copy_runtime_to_scene: fn(&World, &mut World, &Resources, Entity, Entity),
    marker: PhantomData<(Runtime, Scene)>,
}

impl<Runtime: Component + IntoComponent<Scene>, Scene: Component> RuntimeComponent<Runtime, Scene> {
    pub fn copy_runtime_to_scene(
        &self,
        runtime_world: &World,
        scene_world: &mut World,
        resources: &Resources,
        runtime_entity: Entity,
        scene_entity: Entity,
    ) {
        (self.copy_runtime_to_scene)(
            runtime_world,
            scene_world,
            resources,
            runtime_entity,
            scene_entity,
        );
    }
}

impl<Runtime: Component + IntoComponent<Scene>, Scene: Component> FromType<Runtime>
    for RuntimeComponent<Runtime, Scene>
{
    fn from_type() -> Self {
        RuntimeComponent {
            copy_runtime_to_scene: |runtime_world,
                                    scene_world,
                                    resources,
                                    runtime_entity,
                                    scene_entity| {
                let runtime_component = runtime_world.get::<Runtime>(runtime_entity).unwrap();
                let scene_component = runtime_component.into_component(resources);
                scene_world
                    .insert_one(scene_entity, scene_component)
                    .unwrap();
            },
            marker: Default::default(),
        }
    }
}

#[derive(Clone)]
pub struct ReflectMapEntities {
    map_entities: fn(&mut World, &EntityMap) -> Result<(), MapEntitiesError>,
}

impl ReflectMapEntities {
    pub fn map_entities(
        &self,
        world: &mut World,
        entity_map: &EntityMap,
    ) -> Result<(), MapEntitiesError> {
        (self.map_entities)(world, entity_map)
    }
}

impl<C: Component + MapEntities> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |world, entity_map| {
                for entity in entity_map.values() {
                    if let Ok(mut component) = world.get_mut::<C>(entity) {
                        component.map_entities(entity_map)?;
                    }
                }

                Ok(())
            },
        }
    }
}

pub trait IntoComponent<ToComponent: Component> {
    fn into_component(&self, resources: &Resources) -> ToComponent;
}
