#![allow(dead_code)]
use crate::{components::*, ecs::prelude::*};
use smallvec::SmallVec;
use std::collections::HashMap;

pub fn build(_: &mut World) -> Vec<Box<dyn Schedulable>> {
    let missing_previous_parent_system = SystemBuilder::<()>::new("MissingPreviousParentSystem")
        // Entities with missing `PreviousParent`
        .with_query(<Read<Parent>>::query().filter(!component::<PreviousParent>()))
        .build(move |commands, world, _resource, query| {
            // Add missing `PreviousParent` components
            for (entity, _parent) in query.iter_entities(world) {
                log::trace!("Adding missing PreviousParent to {}", entity);
                commands.add_component(entity, PreviousParent(None));
            }
        });

    let parent_update_system = SystemBuilder::<()>::new("ParentUpdateSystem")
        // Entities with a removed `Parent`
        .with_query(<Read<PreviousParent>>::query().filter(!component::<Parent>()))
        // Entities with a changed `Parent`
        .with_query(<(Read<Parent>, Write<PreviousParent>)>::query().filter(changed::<Parent>()))
        // Deleted Parents (ie Entities with `Children` and without a `LocalToWorld`).
        .write_component::<Children>()
        .build(move |commands, world, _resource, queries| {
            // Entities with a missing `Parent` (ie. ones that have a `PreviousParent`), remove
            // them from the `Children` of the `PreviousParent`.
            let (mut children_world, mut world) = world.split::<Write<Children>>();
            for (entity, previous_parent) in queries.0.iter_entities(&mut world) {
                log::trace!("Parent was removed from {}", entity);
                if let Some(previous_parent_entity) = previous_parent.0 {
                    if let Some(mut previous_parent_children) =
                        children_world.get_component_mut::<Children>(previous_parent_entity)
                    {
                        log::trace!(" > Removing {} from it's prev parent's children", entity);
                        previous_parent_children.0.retain(|e| *e != entity);
                    }
                }
            }

            // Tracks all newly created `Children` Components this frame.
            let mut children_additions =
                HashMap::<Entity, SmallVec<[Entity; 8]>>::with_capacity(16);

            // Entities with a changed Parent (that also have a PreviousParent, even if None)
            for (entity, (parent, mut previous_parent)) in queries.1.iter_entities_mut(&mut world) {
                log::trace!("Parent changed for {}", entity);

                // If the `PreviousParent` is not None.
                if let Some(previous_parent_entity) = previous_parent.0 {
                    // New and previous point to the same Entity, carry on, nothing to see here.
                    if previous_parent_entity == parent.0 {
                        log::trace!(" > But the previous parent is the same, ignoring...");
                        continue;
                    }

                    // Remove from `PreviousParent.Children`.
                    if let Some(mut previous_parent_children) =
                        children_world.get_component_mut::<Children>(previous_parent_entity)
                    {
                        log::trace!(" > Removing {} from prev parent's children", entity);
                        (*previous_parent_children).0.retain(|e| *e != entity);
                    }
                }

                // Set `PreviousParent = Parent`.
                *previous_parent = PreviousParent(Some(parent.0));

                // Add to the parent's `Children` (either the real component, or
                // `children_additions`).
                log::trace!("Adding {} to it's new parent {}", entity, parent.0);
                if let Some(mut new_parent_children) = children_world.get_component_mut::<Children>(parent.0)
                {
                    // This is the parent
                    log::trace!(
                        " > The new parent {} already has a `Children`, adding to it.",
                        parent.0
                    );
                    (*new_parent_children).0.push(entity);
                } else {
                    // The parent doesn't have a children entity, lets add it
                    log::trace!(
                        "The new parent {} doesn't yet have `Children` component.",
                        parent.0
                    );
                    children_additions
                        .entry(parent.0)
                        .or_insert_with(Default::default)
                        .push(entity);
                }
            }

            // Flush the `children_additions` to the command buffer. It is stored separate to
            // collect multiple new children that point to the same parent into the same
            // SmallVec, and to prevent redundant add+remove operations.
            children_additions.iter().for_each(|(k, v)| {
                log::trace!("Flushing: Entity {} adding `Children` component {:?}", k, v);
                commands.add_component(*k, Children::with(v));
            });
        });

    vec![missing_previous_parent_system, parent_update_system]
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn correct_children() {
//         let _ = env_logger::builder().is_test(true).try_init();

//         let mut world = Universe::new().create_world();

//         let systems = build(&mut world);

//         // Add parent entities
//         let parent = *world
//             .insert(
//                 (),
//                 vec![(Translation::identity(), LocalToWorld::identity())],
//             )
//             .first()
//             .unwrap();
//         let children = world.insert(
//             (),
//             vec![
//                 (
//                     Translation::identity(),
//                     LocalToParent::identity(),
//                     LocalToWorld::identity(),
//                 ),
//                 (
//                     Translation::identity(),
//                     LocalToParent::identity(),
//                     LocalToWorld::identity(),
//                 ),
//             ],
//         );
//         let (e1, e2) = (children[0], children[1]);

//         // Parent `e1` and `e2` to `parent`.
//         world.add_component(e1, Parent(parent));
//         world.add_component(e2, Parent(parent));

//         for system in systems.iter() {
//             system.run(&mut world);
//             system.command_buffer_mut().write(&mut world);
//         }

//         assert_eq!(
//             world
//                 .get_component::<Children>(parent)
//                 .unwrap()
//                 .0
//                 .iter()
//                 .cloned()
//                 .collect::<Vec<_>>(),
//             vec![e1, e2]
//         );

//         // Parent `e1` to `e2`.
//         (*world.get_component_mut::<Parent>(e1).unwrap()).0 = e2;

//         // Run the system on it
//         for system in systems.iter() {
//             system.run(&mut world);
//             system.command_buffer_mut().write(&mut world);
//         }

//         assert_eq!(
//             world
//                 .get_component::<Children>(parent)
//                 .unwrap()
//                 .0
//                 .iter()
//                 .cloned()
//                 .collect::<Vec<_>>(),
//             vec![e2]
//         );

//         assert_eq!(
//             world
//                 .get_component::<Children>(e2)
//                 .unwrap()
//                 .0
//                 .iter()
//                 .cloned()
//                 .collect::<Vec<_>>(),
//             vec![e1]
//         );

//         world.delete(e1);

//         // Run the system on it
//         for system in systems.iter() {
//             system.run(&mut world);
//             system.command_buffer_mut().write(&mut world);
//         }

//         assert_eq!(
//             world
//                 .get_component::<Children>(parent)
//                 .unwrap()
//                 .0
//                 .iter()
//                 .cloned()
//                 .collect::<Vec<_>>(),
//             vec![e2]
//         );
//     }
// }
