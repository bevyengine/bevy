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
//! For a full list, see the docs for [`ArchetypeInvariant`].
use bevy::prelude::*;

fn main() {
    App::new()
        // Archetype invariants are constructed in terms of either bundles or single components.
        // This invariant ensures that Player and Camera can never be found together on the same entity.
        .add_archetype_invariant(Player::forbids::<Camera>())
        // This invariant ensures that the `GlobalTransform` component is always found with the `Transform` component, and vice versa.
        .add_archetype_invariant(<(GlobalTransform, Transform)>::atomic())
        // This invariant ensures that the `Player` component is always found with the `Life` component.
        // This requirement is only in one direction: it is possible to have entities which have `Life`, but not `Player` (like enemies).
        .add_archetype_invariant(Player::requires::<Life>())
        // The `disjoint` invariant ensures that at most one component from the bundle is present on a given entity.
        // This way, an entity never belongs to more than one RPG class at once.
        // This is useful for creating groups of components that behave similarly to an enum.
        .add_archetype_invariant(<(Archer, Swordsman, Mage)>::disjoint())
        // This invariant indicates that any entity with the `Player` component always has
        // at least one component in the `(Archer, Swordsman, Mage)` bundle.
        // We could use a type alias to improve clarity and avoid errors caused by duplication:
        //   type Class = (Archer, Swordsman, Mage);
        .add_archetype_invariant(Player::requires_one::<(Archer, Swordsman, Mage)>())
        // You can also specify custom invariants by constructing `ArchetypeInvariant` directly.
        // This invariant specifies that the `Node` component cannot appear on any entity in our world.
        // We're not using bevy_ui in our App, so this component should never show up.
        .add_archetype_invariant(ArchetypeInvariant {
            premise: ArchetypeStatement::<Node>::all_of(),
            consequence: ArchetypeStatement::always_false(),
        })
        .add_startup_system(spawn_player)
        .add_system(position_player)
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Life;

#[derive(Component)]
struct Archer;

#[derive(Component)]
struct Swordsman;

#[derive(Component)]
struct Mage;

fn spawn_player(mut commands: Commands) {
    commands.spawn((Player, Mage, Life));
}

fn position_player(mut commands: Commands, query: Query<Entity, Added<Player>>) {
    let player_entity = query.single();

    // Because of our invariants, these components need to be added together.
    // Adding them separately (as in the broken code below) will cause the entity to briefly enter an invalid state,
    // where it has only one of the two components.
    commands
        .entity(player_entity)
        .insert((GlobalTransform::default(), Transform::default()));

    // Adding the components one at a time panics.
    // Track this limitation at https://github.com/bevyengine/bevy/issues/5074.
    /*
    commands
        .entity(player_entity)
        .insert(GlobalTransform::default())
        .insert(Transform::default());
    */
}
