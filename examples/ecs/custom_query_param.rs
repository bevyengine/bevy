use bevy::{
    ecs::{
        component::Component,
        query::{Fetch, FilterFetch},
    },
    prelude::*,
};
use std::{fmt::Debug, marker::PhantomData};

/// This examples illustrates the usage of `Fetch` and `FilterFetch` derive macros, that allow
/// defining custom query and filter types.
///
/// While regular tuple queries work great in most of simple scenarios, using custom queries
/// declared as named structs can bring the following advantages:
/// - They help to avoid destructuring or using `q.0, q.1, ...` access pattern.
/// - Adding, removing components or changing items order with structs greatly reduces maintenance
///   burden, as you don't need to update statements that destructure tuples, care about order
///   of elements, etc. Instead, you can just add or remove places where a certain element is used.
/// - Named structs enable the composition pattern, that makes query types easier to re-use.
/// - You can bypass the limit of 15 components that exists for query tuples.
///
/// For more details on the `Fetch` and `FilterFetch` derive macros, see their documentation.
fn main() {
    App::new()
        .add_startup_system(spawn)
        .add_system(print_components_read_only.label("print_components_read_only"))
        .add_system(
            print_components_iter_mut
                .label("print_components_iter_mut")
                .after("print_components_read_only"),
        )
        .add_system(
            print_components_iter
                .label("print_components_iter")
                .after("print_components_iter_mut"),
        )
        .add_system(print_components_tuple.after("print_components_iter_mut"))
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

#[derive(Fetch)]
struct ReadOnlyCustomQuery<'w, T: Component + Debug, P: Component + Debug> {
    entity: Entity,
    a: &'w ComponentA,
    b: Option<&'w ComponentB>,
    nested: NestedQuery<'w>,
    generic: GenericQuery<'w, T, P>,
    #[allow(dead_code)]
    empty: EmptyQuery<'w>,
}

fn print_components_read_only(
    query: Query<ReadOnlyCustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (read_only):");
    for e in query.iter() {
        let e: ReadOnlyCustomQuery<'_, _, _> = e;
        println!("Entity: {:?}", e.entity);
        println!("A: {:?}", e.a);
        println!("B: {:?}", e.b);
        println!("Nested: {:?}", e.nested);
        println!("Generic: {:?}", e.generic);
    }
    println!();
}

// If you are going to mutate the data in a query, you must mark it with the `mutable` attribute.
// The `Fetch` derive macro will still create a read-only version, which will be have `ReadOnly`
// suffix.
// Note: if you want to use derive macros with read-only query variants, you need to pass them with
// using the `read_only_derive` attribute.
#[derive(Fetch, Debug)]
#[mutable]
#[read_only_derive(Debug)]
struct CustomQuery<'w, T: Component + Debug, P: Component + Debug> {
    entity: Entity,
    // `Mut<'w, T>` is a necessary replacement for `&'w mut T`
    a: Mut<'w, ComponentA>,
    b: Option<Mut<'w, ComponentB>>,
    nested: NestedQuery<'w>,
    generic: GenericQuery<'w, T, P>,
    #[allow(dead_code)]
    empty: EmptyQuery<'w>,
}

// This is a valid query as well, which would iterate over every entity.
#[derive(Fetch, Debug)]
struct EmptyQuery<'w> {
    _w: std::marker::PhantomData<&'w ()>,
}

#[derive(Fetch, Debug)]
#[allow(dead_code)]
struct NestedQuery<'w> {
    c: &'w ComponentC,
    d: Option<&'w ComponentD>,
}

#[derive(Fetch, Debug)]
#[allow(dead_code)]
struct GenericQuery<'w, T: Component, P: Component> {
    generic: (&'w T, &'w P),
}

#[derive(FilterFetch)]
struct QueryFilter<T: Component, P: Component> {
    _c: With<ComponentC>,
    _d: With<ComponentD>,
    _or: Or<(Added<ComponentC>, Changed<ComponentD>, Without<ComponentZ>)>,
    _generic_tuple: (With<T>, With<P>),
    _tp: PhantomData<(T, P)>,
}

fn spawn(mut commands: Commands) {
    commands
        .spawn()
        .insert(ComponentA)
        .insert(ComponentB)
        .insert(ComponentC)
        .insert(ComponentD);
}

fn print_components_iter_mut(
    mut query: Query<CustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (iter_mut):");
    for e in query.iter_mut() {
        println!("Entity: {:?}", e.entity);
        println!("A: {:?}", e.a);
        println!("B: {:?}", e.b);
        println!("Nested: {:?}", e.nested);
        println!("Generic: {:?}", e.generic);
    }
    println!();
}

fn print_components_iter(
    query: Query<CustomQuery<ComponentC, ComponentD>, QueryFilter<ComponentC, ComponentD>>,
) {
    println!("Print components (iter):");
    for e in query.iter() {
        // Note that the actual type is different when you iterate over mutable queries with `iter`.
        let e: CustomQueryReadOnly<'_, _, _> = e;
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
    for (entity, a, b, nested, (generic_c, generic_d)) in query.iter() {
        println!("Entity: {:?}", entity);
        println!("A: {:?}", a);
        println!("B: {:?}", b);
        println!("Nested: {:?} {:?}", nested.0, nested.1);
        println!("Generic: {:?} {:?}", generic_c, generic_d);
    }
}
