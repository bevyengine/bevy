use crate::{
    archetype::ArchetypeCreated, lifecycle::HookContext, prelude::*, world::DeferredWorld,
};

#[derive(Component)]
struct A;

#[derive(Component)]
#[component(on_add = a_on_add, on_insert = a_on_insert, on_replace = a_on_replace, on_remove = a_on_remove)]
struct AMacroHooks;

fn a_on_add(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(0);
}

fn a_on_insert(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(1);
}

fn a_on_replace(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(2);
}

fn a_on_remove(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(3);
}

#[derive(Component)]
struct B;

#[derive(Component)]
struct C;

#[derive(Component)]
struct D;

#[derive(Component, Eq, PartialEq, Debug)]
struct V(&'static str); // component with a value

#[derive(Resource, Default)]
struct R(usize);

impl R {
    #[track_caller]
    fn assert_order(&mut self, count: usize) {
        assert_eq!(count, self.0);
        self.0 += 1;
    }
}

#[derive(Bundle)]
#[bundle(ignore_from_components)]
struct BundleNoExtract {
    b: B,
    no_from_comp: crate::spawn::SpawnRelatedBundle<ChildOf, Spawn<C>>,
}

#[test]
fn can_spawn_bundle_without_extract() {
    let mut world = World::new();
    let id = world
        .spawn(BundleNoExtract {
            b: B,
            no_from_comp: Children::spawn(Spawn(C)),
        })
        .id();

    assert!(world.entity(id).get::<Children>().is_some());
}

#[test]
fn component_hook_order_spawn_despawn() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
        .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(2))
        .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

    let entity = world.spawn(A).id();
    world.despawn(entity);
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_spawn_despawn_with_macro_hooks() {
    let mut world = World::new();
    world.init_resource::<R>();

    let entity = world.spawn(AMacroHooks).id();
    world.despawn(entity);

    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_insert_remove() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| world.resource_mut::<R>().assert_order(1))
        .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(2))
        .on_remove(|mut world, _| world.resource_mut::<R>().assert_order(3));

    let mut entity = world.spawn_empty();
    entity.insert(A);
    entity.remove::<A>();
    entity.flush();
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_replace() {
    let mut world = World::new();
    world
        .register_component_hooks::<A>()
        .on_replace(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| {
            if let Some(mut r) = world.get_resource_mut::<R>() {
                r.assert_order(1);
            }
        });

    let entity = world.spawn(A).id();
    world.init_resource::<R>();
    let mut entity = world.entity_mut(entity);
    entity.insert(A);
    entity.insert_if_new(A); // this will not trigger on_replace or on_insert
    entity.flush();
    assert_eq!(2, world.resource::<R>().0);
}

#[test]
fn component_hook_order_recursive() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(0);
            world.commands().entity(context.entity).insert(B);
        })
        .on_remove(|mut world, context| {
            world.resource_mut::<R>().assert_order(2);
            world.commands().entity(context.entity).remove::<B>();
        });

    world
        .register_component_hooks::<B>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(1);
            world.commands().entity(context.entity).remove::<A>();
        })
        .on_remove(|mut world, _| {
            world.resource_mut::<R>().assert_order(3);
        });

    let entity = world.spawn(A).flush();
    let entity = world.get_entity(entity).unwrap();
    assert!(!entity.contains::<A>());
    assert!(!entity.contains::<B>());
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn component_hook_order_recursive_multiple() {
    let mut world = World::new();
    world.init_resource::<R>();
    world
        .register_component_hooks::<A>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(0);
            world.commands().entity(context.entity).insert(B).insert(C);
        });

    world
        .register_component_hooks::<B>()
        .on_add(|mut world, context| {
            world.resource_mut::<R>().assert_order(1);
            world.commands().entity(context.entity).insert(D);
        });

    world
        .register_component_hooks::<C>()
        .on_add(|mut world, _| {
            world.resource_mut::<R>().assert_order(3);
        });

    world
        .register_component_hooks::<D>()
        .on_add(|mut world, _| {
            world.resource_mut::<R>().assert_order(2);
        });

    world.spawn(A).flush();
    assert_eq!(4, world.resource::<R>().0);
}

#[test]
fn insert_if_new() {
    let mut world = World::new();
    let id = world.spawn(V("one")).id();
    let mut entity = world.entity_mut(id);
    entity.insert_if_new(V("two"));
    entity.insert_if_new((A, V("three")));
    entity.flush();
    // should still contain "one"
    let entity = world.entity(id);
    assert!(entity.contains::<A>());
    assert_eq!(entity.get(), Some(&V("one")));
}

#[derive(Component, Debug, Eq, PartialEq)]
#[component(storage = "SparseSet")]
pub struct SparseV(&'static str);

#[derive(Component, Debug, Eq, PartialEq)]
#[component(storage = "SparseSet")]
pub struct SparseA;

#[test]
fn sparse_set_insert_if_new() {
    let mut world = World::new();
    let id = world.spawn(SparseV("one")).id();
    let mut entity = world.entity_mut(id);
    entity.insert_if_new(SparseV("two"));
    entity.insert_if_new((SparseA, SparseV("three")));
    entity.flush();
    // should still contain "one"
    let entity = world.entity(id);
    assert!(entity.contains::<SparseA>());
    assert_eq!(entity.get(), Some(&SparseV("one")));
}

#[test]
fn new_archetype_created() {
    let mut world = World::new();
    #[derive(Resource, Default)]
    struct Count(u32);
    world.init_resource::<Count>();
    world.add_observer(|_t: On<ArchetypeCreated>, mut count: ResMut<Count>| {
        count.0 += 1;
    });

    let mut e = world.spawn((A, B));
    e.insert(C);
    e.remove::<A>();
    e.insert(A);
    e.insert(A);

    assert_eq!(world.resource::<Count>().0, 3);
}

#[derive(Bundle)]
#[expect(unused, reason = "tests the output of the derive macro is valid")]
struct Ignore {
    #[bundle(ignore)]
    foo: i32,
    #[bundle(ignore)]
    bar: i32,
}
