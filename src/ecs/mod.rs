pub mod default_archetypes;
mod entity_archetype;
mod world_builder;

pub use entity_archetype::*;
pub use world_builder::*;

use bevy_transform::prelude::Children;
use legion::{
    prelude::{Entity, World},
    system::SubWorld,
};

pub fn run_on_hierarchy<T>(
    world: &World,
    entity: Entity,
    input: T,
    func: &mut dyn FnMut(&World, Entity, T) -> Option<T>,
) where
    T: Copy,
{
    let result = func(world, entity, input);

    if let Some(result) = result {
        match world.get_component::<Children>(entity) {
            Some(children) => Some(for child in children.iter() {
                run_on_hierarchy(world, *child, result, func);
            }),
            None => None,
        };
    }
}

pub fn run_on_hierarchy_mut<T>(
    world: &mut World,
    entity: Entity,
    input: T,
    func: &mut dyn FnMut(&mut World, Entity, T) -> Option<T>,
) where
    T: Copy,
{
    // TODO: not a huge fan of this pattern. are there ways to do recursive updates in legion without allocactions?
    let children = match world.get_component::<Children>(entity) {
        Some(children) => Some(
            children
                .iter()
                .map(|entity| *entity)
                .collect::<Vec<Entity>>(),
        ),
        None => None,
    };

    let result = func(world, entity, input);

    if let Some(result) = result {
        if let Some(children) = children {
            for child in children {
                run_on_hierarchy_mut(world, child, result, func);
            }
        }
    }
}

pub fn run_on_hierarchy_subworld<T>(
    world: &SubWorld,
    entity: Entity,
    input: T,
    func: &mut dyn FnMut(&SubWorld, Entity, T) -> Option<T>,
) where
    T: Copy,
{
    let result = func(world, entity, input);

    if let Some(result) = result {
        match world.get_component::<Children>(entity) {
            Some(children) => Some(for child in children.iter() {
                run_on_hierarchy_subworld(world, *child, result, func);
            }),
            None => None,
        };
    }
}

pub fn run_on_hierarchy_subworld_mut<T>(
    world: &mut SubWorld,
    entity: Entity,
    input: T,
    func: &mut dyn FnMut(&mut SubWorld, Entity, T) -> Option<T>,
) where
    T: Copy,
{
    // TODO: not a huge fan of this pattern. are there ways to do recursive updates in legion without allocactions?
    let children = match world.get_component::<Children>(entity) {
        Some(children) => Some(
            children
                .iter()
                .map(|entity| *entity)
                .collect::<Vec<Entity>>(),
        ),
        None => None,
    };

    let result = func(world, entity, input);

    if let Some(result) = result {
        if let Some(children) = children {
            for child in children {
                run_on_hierarchy_subworld_mut(world, child, result, func);
            }
        }
    }
}
