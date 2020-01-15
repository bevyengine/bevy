use crate::prelude::{Children, Entity, SubWorld, World};

pub fn run_on_hierarchy<T>(
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
                run_on_hierarchy(world, child, result, func);
            }
        }
    }
}

pub fn run_on_hierarchy_subworld<T>(
    world: &mut legion::system::SubWorld,
    entity: Entity,
    input: T,
    func: &dyn Fn(&mut SubWorld, Entity, T) -> Option<T>,
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
                run_on_hierarchy_subworld(world, child, result, func);
            }
        }
    }
}
