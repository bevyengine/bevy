// extern crate legion;
// extern crate legion_transform;

// use legion::prelude::*;
// use legion_transform::prelude::*;
fn main() {}

// #[allow(unused)]
// fn tldr_sample() {
//     // Create a normal Legion World
//     let mut world = Universe::default().create_world();

//     // Create a system bundle (vec of systems) for LegionTransform
//     let transform_system_bundle = transform_system_bundle::build(&mut world);

//     let parent_entity = *world
//         .insert(
//             (),
//             vec![(
//                 // Always needed for an Entity that has any space transform
//                 LocalToWorld::identity(),
//                 // The only mutable space transform a parent has is a translation.
//                 Translation::new(100.0, 0.0, 0.0),
//             )],
//         )
//         .first()
//         .unwrap();

//     world.insert(
//         (),
//         vec![
//             (
//                 // Again, always need a `LocalToWorld` component for the Entity to have a custom
//                 // space transform.
//                 LocalToWorld::identity(),
//                 // Here we define a Translation, Rotation and uniform Scale.
//                 Translation::new(1.0, 2.0, 3.0),
//                 Rotation::from_euler_angles(3.14, 0.0, 0.0),
//                 Scale(2.0),
//                 // Add a Parent and LocalToParent component to attach a child to a parent.
//                 Parent(parent_entity),
//                 LocalToParent::identity(),
//             );
//             4
//         ],
//     );
// }

// fn main() {
//     // Create a normal Legion World
//     let mut world = Universe::default().create_world();

//     // Create a system bundle (vec of systems) for LegionTransform
//     let transform_system_bundle = transform_system_bundle::build(&mut world);

//     // See `./types_of_transforms.rs` for an explanation of space-transform types.
//     let parent_entity = *world
//         .insert(
//             (),
//             vec![(LocalToWorld::identity(), Translation::new(100.0, 0.0, 0.0))],
//         )
//         .first()
//         .unwrap();

//     let four_children: Vec<_> = world
//         .insert(
//             (),
//             vec![
//                 (
//                     LocalToWorld::identity(),
//                     Translation::new(1.0, 2.0, 3.0),
//                     Rotation::from_euler_angles(3.14, 0.0, 0.0),
//                     Scale(2.0),
//                     // Add a Parent and LocalToParent component to attach a child to a parent.
//                     Parent(parent_entity),
//                     LocalToParent::identity(),
//                 );
//                 4
//             ],
//         )
//         .iter()
//         .cloned()
//         .collect();

//     // At this point the parent does NOT have a `Children` component attached to it. The `Children`
//     // component is updated by the transform system bundle and thus can be out of date (or
//     // non-existent for newly added members). By this logic, the `Parent` components should be
//     // considered the always-correct 'source of truth' for any hierarchy.
//     for system in transform_system_bundle.iter() {
//         system.run(&mut world);
//         system.command_buffer_mut().write(&mut world);
//     }

//     // At this point all parents with children have a correct `Children` component.
//     let parents_children = world
//         .get_component::<Children>(parent_entity)
//         .unwrap()
//         .0
//         .clone();

//     println!("Parent {}", parent_entity);
//     for child in parents_children.iter() {
//         println!(" -> Has child: {}", child);
//     }

//     // Each child will also have a `LocalToParent` component attached to it, which is a
//     // space-transform from its local space to that of its parent.
//     for child in four_children.iter() {
//         println!("The child {}", child);
//         println!(
//             " -> Has a LocalToParent matrix: {}",
//             *world.get_component::<LocalToParent>(*child).unwrap()
//         );
//         println!(
//             " -> Has a LocalToWorld matrix: {}",
//             *world.get_component::<LocalToWorld>(*child).unwrap()
//         );
//     }

//     // Re-parent the second child to be a grandchild of the first.
//     world.add_component(four_children[1], Parent(four_children[0]));

//     // Re-running the system will cleanup and fix all `Children` components.
//     for system in transform_system_bundle.iter() {
//         system.run(&world);
//         system.command_buffer_mut().write(&mut world);
//     }

//     println!("After the second child was re-parented as a grandchild of the first child...");

//     for child in world
//         .get_component::<Children>(parent_entity)
//         .unwrap()
//         .0
//         .iter()
//     {
//         println!("Parent {} has child: {}", parent_entity, child);
//     }

//     for grandchild in world
//         .get_component::<Children>(four_children[0])
//         .unwrap()
//         .0
//         .iter()
//     {
//         println!("Child {} has grandchild: {}", four_children[0], grandchild);
//     }

//     println!("Grandchild: {}", four_children[1]);
//     println!(
//         " -> Has a LocalToWorld matrix: {}",
//         *world
//             .get_component::<LocalToWorld>(four_children[1])
//             .unwrap()
//     );
// }
