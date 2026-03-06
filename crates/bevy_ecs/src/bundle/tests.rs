use crate::{
    archetype::ArchetypeCreated, lifecycle::HookContext, prelude::*, world::DeferredWorld,
};

#[derive(Component)]
struct A;

#[derive(Component)]
#[component(on_add = a_on_add, on_insert = a_on_insert, on_discard = a_on_discard, on_remove = a_on_remove)]
struct AMacroHooks;

fn a_on_add(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(0);
}

fn a_on_insert(mut world: DeferredWorld, _: HookContext) {
    world.resource_mut::<R>().assert_order(1);
}

fn a_on_discard(mut world: DeferredWorld, _: HookContext) {
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
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(2))
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
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(2))
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
        .on_discard(|mut world, _| world.resource_mut::<R>().assert_order(0))
        .on_insert(|mut world, _| {
            if let Some(mut r) = world.get_resource_mut::<R>() {
                r.assert_order(1);
            }
        });

    let entity = world.spawn(A).id();
    world.init_resource::<R>();
    let mut entity = world.entity_mut(entity);
    entity.insert(A);
    entity.insert_if_new(A); // this will not trigger on_discard or on_insert
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

#[test]
fn table_table_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn sparse_sparse_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn table_sparse_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn sparse_table_mutually_exclusive_component_removes_other() {
    let mut world = World::new();
    #[derive(Component, Default)]
    #[component(storage = "SparseSet")]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
}

#[test]
fn mutually_exclusive_component_hooks_run() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Resource, Default, PartialEq, Eq, Debug)]
    struct Counter {
        add: u8,
        insert: u8,
        discard: u8,
        remove: u8,
    }

    world.init_resource::<Counter>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world
        .register_component_hooks::<CompA>()
        .on_add(|mut world, _| world.resource_mut::<Counter>().add += 1)
        .on_insert(|mut world, _| world.resource_mut::<Counter>().insert += 1)
        .on_discard(|mut world, _| world.resource_mut::<Counter>().discard += 1)
        .on_remove(|mut world, _| world.resource_mut::<Counter>().remove += 1);

    let e = world.spawn(CompA).id();

    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    assert_eq!(
        &Counter {
            add: 1,
            insert: 1,
            discard: 1,
            remove: 1
        },
        world.resource::<Counter>()
    );

    world.entity_mut(e).insert(CompA);
    assert!(!world.entity(e).contains::<CompB>());
    assert_eq!(
        &Counter {
            add: 2,
            insert: 2,
            discard: 1,
            remove: 1
        },
        world.resource::<Counter>()
    );
}

#[test]
#[should_panic]
fn mutually_exclusive_spawn_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    world.spawn((CompA, CompB));
}

#[test]
fn mutually_exclusive_required_removes_other() {
    let mut world = World::new();

    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    // Depth 1
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompB, CompC>();

    #[derive(Component, Default)]
    struct CompD;

    #[derive(Component, Default)]
    struct CompE;

    #[derive(Component, Default)]
    struct CompF;

    // Depth 2
    world.register_mutually_exclusive_components::<(CompA, CompF)>();
    world.register_required_components::<CompD, CompE>();
    world.register_required_components::<CompE, CompF>();

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompB);
    assert!(!world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompB>());
    assert!(world.entity(e).contains::<CompC>());

    let e = world.spawn(CompA).id();
    world.entity_mut(e).insert(CompD);
    assert!(!world.entity(e).contains::<CompA>());
    assert!(world.entity(e).contains::<CompF>());
}

#[test]
#[should_panic]
fn registering_required_as_mutually_exclusive_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_required_components::<CompA, CompB>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_as_required_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompA, CompB>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_after_any_archetype_contains_target_components_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    world.spawn(CompA);
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
}

#[test]
#[should_panic]
fn registering_mutually_exclusive_as_indirect_required_panics() {
    let mut world = World::new();
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    world.register_mutually_exclusive_components::<(CompB, CompC)>();
    world.register_required_components::<CompA, CompB>();
    world.register_required_components::<CompA, CompC>();
}

#[test]
fn mutually_exclusive_with_same_required() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    // Required first
    let mut world = World::new();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompC>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    // Mutually exclusive first
    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompC>();
}

#[test]
#[should_panic]
fn mutually_exclusive_within_required_panics_on_spawn() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    #[derive(Component, Default)]
    struct CompC;

    #[derive(Component, Default)]
    struct CompD;

    #[derive(Component, Default)]
    struct CompE;

    let mut world = World::new();
    world.register_mutually_exclusive_components::<(CompC, CompE)>();
    world.register_required_components::<CompA, CompC>();
    world.register_required_components::<CompB, CompD>();
    world.register_required_components::<CompD, CompE>();

    world.spawn((CompA, CompB));
}

#[test]
#[should_panic]
fn mutually_exclusive_after_bundle_register_panics_on_spawn() {
    #[derive(Component, Default)]
    struct CompA;

    #[derive(Component, Default)]
    struct CompB;

    let mut world = World::new();
    world.register_bundle::<(CompA, CompB)>();
    world.register_mutually_exclusive_components::<(CompA, CompB)>();

    world.spawn((CompA, CompB));
}
