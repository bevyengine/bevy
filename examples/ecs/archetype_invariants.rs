//! Archetype invariants are rules about which combinations of components can coexist.
//!
//! An archetype (in the sense that Bevy uses it) is the "unique set of components" that belong to an entity.
//! These are useful to codify your assumptions about the composition of your entities.
//! For example, an entity can never have a `Player` component with a `Camera` together,
//! or a `GlobalTransform` may only be valid in association with a `Transform`.
//! By constructing `ArchetypeInvariant`s out of `ArchetypeStatement`s,
//! we can encode this logic into our app.
//!
//! Archetype invariants are guaranteed to hold at *all* points during the app's lifecycle;
//! this is automtically checked on component insertion and removal, including when entities are spawned.
//! Make sure to test thoroughly when using archetype invariants in production though;
//! any violations will result in a panic!
//!
//! There are many helper methods provided on `ArchetypeInvariant` to help easily construct common invariant patterns,
//! but we will only be showcasing some of them here.
//! For a full list, see the docs for `ArchetypeInvariant`.
use bevy::prelude::*;

fn main() {
    App::new()
        // Archetype invariants are constructed in terms of bundles;
        // use (MyComponent, ) to construct a bundle with a single item.
        // This invariant ensures that Player and Camera can never be found together.
        .add_archetype_invariant(ArchetypeInvariant::<(Player,), (Camera,)>::forbids())
        // This invariant ensures that the presence of a `GlobalTransform` component implies the existence of a `Transform` component.
        .add_archetype_invariant(ArchetypeInvariant::<(GlobalTransform,), (Transform,)>::requires())
        // Note that the converse statement isn't automatically true!
        // With only the above invariant, a entity with only `Transform` is valid.
        // To fix this, swap the order of the generic types and add a new invariant.
        .add_archetype_invariant(ArchetypeInvariant::<(Transform,), (GlobalTransform,)>::requires())
        // The `disjoint` invariant ensures that at most one component from the bundle is present on a given entity.
        // This way, an entity can never be an animal and a vegetable at once.
        // This is useful for creating groups of components that behave conceptually similarly to an enum.
        .add_archetype_invariant(ArchetypeInvariant::<(Animal, Vegetable, Mineral)>::disjoint())
        // You can also specify custom invariants by constructing `ArchetypeInvariant` directly.
        // This invariant ensures that all entities have at least one component from the bundle given.
        // Combined with the above invariant, this means that every entity has exactly one of (Animal, Vegetable, Mineral).
        .add_archetype_invariant(ArchetypeInvariant {
            // This statement is always true, and so matches all entities regardless of their archetype.
            premise: ArchetypeStatement::always_true(),
            // ArchetypeStatement::AnyOf evaluates to true when at least one of the components is present on the entity.
            consequence: ArchetypeStatement::<(Animal, Vegetable, Mineral)>::any_of(),
        })
        .add_startup_system(spawn_vegetable)
        .add_system(position_vegetable)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Animal;

#[derive(Component)]
struct Vegetable;

#[derive(Component)]
struct Mineral;

fn spawn_vegetable(mut commands: Commands) {
    commands.spawn().insert(Vegetable);
}

fn position_vegetable(mut commands: Commands, query: Query<Entity, With<Vegetable>>) {
    let vegetable_entity = query.single();

    // Because of our invariants, these components need to be added together.
    // Adding them separately (as in the broken code below) will cause the entity to briefly enter an invalid state,
    // where it has only one of the two components.
    commands
        .entity(vegetable_entity)
        .insert_bundle((GlobalTransform::default(), Transform::default()));

    // Adding the components one at a time panics
    // Track this limitation at https://github.com/bevyengine/bevy/issues/5074
    /*
    commands
        .entity(vegetable_entity)
        .insert(GlobalTransform::default())
        .insert(Transform::default());
    */
}
