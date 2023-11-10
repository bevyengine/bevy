//! This example illustrates the usage of the [`WorldQuery`] derive macro, which allows
//! defining custom query and filter types.
//!
//! While regular tuple queries work great in most of simple scenarios, using custom queries
//! declared as named structs can bring the following advantages:
//! - They help to avoid destructuring or using `q.0, q.1, ...` access pattern.
//! - Adding, removing components or changing items order with structs greatly reduces maintenance
//!   burden, as you don't need to update statements that destructure tuples, care about order
//!   of elements, etc. Instead, you can just add or remove places where a certain element is used.
//! - Named structs enable the composition pattern, that makes query types easier to re-use.
//! - You can bypass the limit of 15 components that exists for query tuples.
//!
//! For more details on the `WorldQuery` derive macro, see the trait documentation.

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use bevy::{ecs::query::WorldQuery, prelude::*};
use std::fmt::Debug;

fn main() {
    App::new()
        .add_systems(Startup, spawn)
        .add_systems(
            Update,
            (
                print_components_read_only,
                print_components_iter_mut,
                print_components_iter,
                print_components_tuple,
            )
                .chain(),
        )
        .run();
}

#[derive(Component, Debug)]
struct ComponentA;
#[derive(Component, Debug)]
struct ComponentB;
#[derive(Component, Debug)]
struct ComponentC;
#[derive(Component, Debug)]
struct ComponentD;
#[derive(Component, Debug)]
struct ComponentZ;

#[derive(WorldQuery)]
#[world_query(derive(Debug))]
struct ReadOnlyCustomQuery<T: Component + Debug, P: Component + Debug> {
    entity: Entity,
    a: &'static ComponentA,
    b: Option<&'static ComponentB>,
    nested: NestedQuery,
    optional_nested: Option<NestedQuery>,
    optional_tuple: Option<(&'static ComponentB, &'static ComponentZ)>,
    generic: GenericQuery<T, P>,
    empty: EmptyQuery,
}

fn print_components_read_only(
    query: Query<ReadOnlyCustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (read_only):");
    for e in &query {
        println!("Entity: {:?}", e.entity);
        println!("A: {:?}", e.a);
        println!("B: {:?}", e.b);
        println!("Nested: {:?}", e.nested);
        println!("Optional nested: {:?}", e.optional_nested);
        println!("Optional tuple: {:?}", e.optional_tuple);
        println!("Generic: {:?}", e.generic);
    }
    println!();
}

// If you are going to mutate the data in a query, you must mark it with the `mutable` attribute.
// The `WorldQuery` derive macro will still create a read-only version, which will be have `ReadOnly`
// suffix.
// Note: if you want to use derive macros with read-only query variants, you need to pass them with
// using the `derive` attribute.
#[derive(WorldQuery)]
#[world_query(mutable, derive(Debug))]
struct CustomQuery<T: Component + Debug, P: Component + Debug> {
    entity: Entity,
    a: &'static mut ComponentA,
    b: Option<&'static mut ComponentB>,
    nested: NestedQuery,
    optional_nested: Option<NestedQuery>,
    optional_tuple: Option<(NestedQuery, &'static mut ComponentZ)>,
    generic: GenericQuery<T, P>,
    empty: EmptyQuery,
}

// This is a valid query as well, which would iterate over every entity.
#[derive(WorldQuery)]
#[world_query(derive(Debug))]
struct EmptyQuery {
    empty: (),
}

#[derive(WorldQuery)]
#[world_query(derive(Debug))]
struct NestedQuery {
    c: &'static ComponentC,
    d: Option<&'static ComponentD>,
}

#[derive(WorldQuery)]
#[world_query(derive(Debug))]
struct GenericQuery<T: Component, P: Component> {
    generic: (&'static T, &'static P),
}

#[derive(WorldQuery)]
struct QueryFilter<T: Component, P: Component> {
    _c: With<ComponentC>,
    _d: With<ComponentD>,
    _or: Or<(Added<ComponentC>, Changed<ComponentD>, Without<ComponentZ>)>,
    _generic_tuple: (With<T>, With<P>),
}

fn spawn(mut commands: Commands) {
    commands.spawn((ComponentA, ComponentB, ComponentC, ComponentD));
}

fn print_components_iter_mut(
    mut query: Query<CustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (iter_mut):");
    for e in &mut query {
        // Re-declaring the variable to illustrate the type of the actual iterator item.
        let e: CustomQueryItem<'_, _, _> = e;
        println!("Entity: {:?}", e.entity);
        println!("A: {:?}", e.a);
        println!("B: {:?}", e.b);
        println!("Optional nested: {:?}", e.optional_nested);
        println!("Optional tuple: {:?}", e.optional_tuple);
        println!("Nested: {:?}", e.nested);
        println!("Generic: {:?}", e.generic);
    }
    println!();
}

fn print_components_iter(
    query: Query<CustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (iter):");
    for e in &query {
        // Re-declaring the variable to illustrate the type of the actual iterator item.
        let e: CustomQueryReadOnlyItem<'_, _, _> = e;
        println!("Entity: {:?}", e.entity);
        println!("A: {:?}", e.a);
        println!("B: {:?}", e.b);
        println!("Nested: {:?}", e.nested);
        println!("Generic: {:?}", e.generic);
    }
    println!();
}

type NestedTupleQuery<'w> = (&'w ComponentC, &'w ComponentD);
type GenericTupleQuery<'w, T, P> = (&'w T, &'w P);

fn print_components_tuple(
    query: Query<
        (
            Entity,
            &ComponentA,
            &ComponentB,
            NestedTupleQuery,
            GenericTupleQuery<ComponentC, ComponentD>,
        ),
        (
            With<ComponentC>,
            With<ComponentD>,
            Or<(Added<ComponentC>, Changed<ComponentD>, Without<ComponentZ>)>,
        ),
    >,
) {
    println!("Print components (tuple):");
    for (entity, a, b, nested, (generic_c, generic_d)) in &query {
        println!("Entity: {entity:?}");
        println!("A: {a:?}");
        println!("B: {b:?}");
        println!("Nested: {:?} {:?}", nested.0, nested.1);
        println!("Generic: {generic_c:?} {generic_d:?}");
    }
}
