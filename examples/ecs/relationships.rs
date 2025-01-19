//! Entities generally don't exist in isolation. Instead, they are related to other entities in various ways.
//! While Bevy comes with a built-in [`Parent`]/[`Children`] relationship
//! (which enables transform and visibility propagation),
//! you can define your own relationships using components.
//!
//! Every relation has two sides: the source and the target.
//! The source is the entity that has the relationship with the target,
//! while the target keeps track of all the entities that have a relationship with it.
//! For the standard hierarchy, the source is stored in the [`Parent`] component,
//! while the target is stored in the [`Children`] component.
//!
//! We can define a custom relationship by creating two components:
//! one to store "what is being targeted" and another to store "what is targeting."
//! In this example we're using the literal names [`Targeting`] and [`TargetedBy`],
//! as games often have units that target other units in combat.

use bevy::ecs::entity::EntityHashSet;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

/// The entity that this entity is targeting.
///
/// This is the source of truth for the relationship,
/// and can be modified directly to change the target.
#[derive(Component, Debug)]
#[relationship(relationship_target = TargetedBy)]
struct Targeting(Entity);

/// All entities that are targeting this entity.
///
/// This component is updated reactively using the component hooks introduced by deriving
/// the [`Relationship`] trait. We should not modify this component directly,
/// but can safely read its field. In a larger project, we could enforce this through the use of
/// private fields and public getters.
#[derive(Component, Debug)]
#[relationship_target(relationship = Targeting)]
struct TargetedBy(Vec<Entity>);

fn main() {
    // Operating on a raw `World` and running systems one at a time
    // is great for writing tests and teaching abstract concepts!
    let mut world = World::new();

    // We're going to spawn a few entities and relate them to each other in a complex way.
    // To start, Alice will target Bob, Bob will target Charlie,
    // and Charlie will target Alice. This creates a loop in the relationship graph.
    //
    // Then, we'll spawn Devon, who will target Charlie,
    // creating a more complex graph with a branching structure.
    fn spawning_entities_with_relationships(mut commands: Commands) {
        // Calling .id() after spawning an entity will return the `Entity` identifier of the spawned entity,
        // even though the entity itself is not yet instantiated in the world.
        // This works because Commands will reserve the entity ID before actually spawning the entity,
        // through the use of atomic counters.
        let alice = commands.spawn(Name::new("Alice")).id();
        // Relations are just components, so we can add them into the bundle that we're spawning.
        let bob = commands.spawn((Name::new("Bob"), Targeting(alice))).id();

        // The `with_related` helper method on `EntityCommands` can be used to add relations in a more ergonomic way.
        let charlie = commands
            .spawn((Name::new("Charlie"), Targeting(bob)))
            // The `with_related` method will automatically add the `Targeting` component to any entities spawned within the closure,
            // targeting the entity that we're calling `with_related` on.
            .with_related::<Targeting>(|related_spawner_commands| {
                // We could spawn multiple entities here, and they would all target `charlie`.
                related_spawner_commands.spawn(Name::new("Devon"));
            })
            .id();

        // Simply inserting the `Targeting` component will automatically create and update the `TargetedBy` component on the target entity.
        // We can do this at any point; not just when the entity is spawned.
        commands.entity(alice).insert(Targeting(charlie));
    }

    world
        .run_system_once(spawning_entities_with_relationships)
        .unwrap();

    fn debug_relationships(
        // Not all of our entities are targeted by something, so we use `Option` in our query to handle this case.
        relations_query: Query<(&Name, &Targeting, Option<&TargetedBy>)>,
        name_query: Query<&Name>,
    ) {
        let mut relationships = String::new();

        for (name, targeting, maybe_targeted_by) in relations_query.iter() {
            let targeting_name = name_query.get(targeting.0).unwrap();
            let targeted_by_string = if let Some(targeted_by) = maybe_targeted_by {
                let mut vec_of_names = Vec::<&Name>::new();

                for entity in &targeted_by.0 {
                    let name = name_query.get(*entity).unwrap();
                    vec_of_names.push(name);
                }

                // Convert this to a nice string for printing.
                let vec_of_str: Vec<&str> = vec_of_names.iter().map(|name| name.as_str()).collect();
                vec_of_str.join(", ")
            } else {
                "nobody".to_string()
            };

            relationships.push_str(&format!(
                "{name} is targeting {targeting_name}, and is targeted by {targeted_by_string}\n",
            ));
        }

        println!("{}", relationships);
    }

    world.run_system_once(debug_relationships).unwrap();

    // Systems can return errors,
    // which can be used to signal that something went wrong during the system's execution.
    #[derive(Debug)]
    struct TargetingCycle {
        initial_entity: Entity,
        visited: EntityHashSet,
    }

    /// Bevy's relationships come with all sorts of useful methods for traversal.
    /// Here, we're going to look for cycles using a depth-first search.
    fn check_for_cycles(
        // We want to check every entity for cycles
        query_to_check: Query<(Entity, &Name), With<Targeting>>,
        // The targeting_query allows us to traverse the relationship graph.
        targeting_query: Query<&Targeting>,
    ) -> Result<(), TargetingCycle> {
        for (initial_entity, initial_entity_name) in query_to_check.iter() {
            println!("Checking for cycles starting from {initial_entity_name}...",);
            let mut visited = EntityHashSet::new();
            let mut targeting_name = initial_entity_name;

            // There's all sorts of methods like this; check the `Query` docs for more!
            // This would also be easy to do by just manually checking the `Targeting` component,
            // and calling `query.get(targeted_entity)` on the entity that it targets in a loop.
            for targeting in targeting_query.iter_ancestors(initial_entity) {
                let target_name = query_to_check.get(targeting).unwrap().1;
                println!("{targeting_name} is targeting {target_name}",);
                targeting_name = target_name;

                if visited.contains(&targeting) {
                    return Err(TargetingCycle {
                        initial_entity,
                        visited,
                    });
                } else {
                    visited.insert(targeting);
                }
            }
        }

        // If we've checked all the entities and haven't found a cycle, we're good!
        Ok(())
    }

    // Calling `world.run_system_once` on systems which return Results gives us two layers of errors:
    // the first checks if running the system failed, and the second checks if the system itself returned an error.
    // We're unwrapping the first, but checking the output of the system itself.
    let cycle_result = world.run_system_once(check_for_cycles).unwrap();
    println!("{:?}", cycle_result);
    // We deliberately introduced a cycle during spawning!
    assert!(cycle_result.is_err());
}
