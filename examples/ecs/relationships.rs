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
        let alice = commands.spawn((Name::new("Alice"))).id();
        // Relations are just components, so we can add them into the bundle that we're spawning.
        let bob = commands.spawn((Name::new("Bob"), Targeting(alice))).id();

        // Simply inserting the `Targeting` component will automatically create and update the `TargetedBy` component on the target entity.
        // We can do this at any point; not just when the entity is spawned.
        commands.entity(alice).insert(Targeting(bob));

        // The `with_related` helper method on `EntityCommands` can be used to add relations in a more ergonomic way.
        let charlie = commands
            .spawn((Name::new("Charlie"), Targeting(bob)))
            // The `with_related` method will automatically add the `Targeting` component to any entities spawned within the closure,
            // targeting the entity that we're calling `with_related` on.
            .with_related::<Targeting>(|related_spawner_commands| {
                related_spawner.spawn(Name::new("Devon"));
            })
            .id();
    }

    world.run_system(spawning_entities_with_relationships);

    fn debug_relationships(query: Query<(&Name, &Targeting, &TargetedBy)>) {
        let mut relationships = String::new();

        for (name, targeting, targeted_by) in query.iter() {
            relationships.push_str(&format!(
                "{} is targeting {:?}, and is targeted by {:?}\n",
                name.0, targeting.0, targeted_by.0
            ));
        }
    }

    world.run_system(debug_relationships);
}
