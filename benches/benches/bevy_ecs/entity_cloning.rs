use bevy_ecs::bundle::Bundle;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::{component::Component, reflect::ReflectComponent, world::World};
use bevy_hierarchy::{BuildChildren, CloneEntityHierarchyExt};
use bevy_math::Mat4;
use bevy_reflect::{GetTypeRegistration, Reflect};
use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};

criterion_group!(benches, reflect_benches, clone_benches);
criterion_main!(benches);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent1(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent2(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent3(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent4(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent5(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent6(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent7(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent8(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent9(Mat4);

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct ReflectComponent10(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent1(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent2(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent3(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent4(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent5(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent6(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent7(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent8(Mat4);
#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent9(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct CloneComponent10(Mat4);

fn hierarchy<C: Bundle + Default + GetTypeRegistration>(
    b: &mut Bencher,
    width: usize,
    height: usize,
) {
    let mut world = World::default();
    let registry = AppTypeRegistry::default();
    {
        let mut r = registry.write();
        r.register::<C>();
    }
    world.insert_resource(registry);

    let id = world.spawn(black_box(C::default())).id();

    let mut hierarchy_level = vec![id];

    for _ in 0..height {
        let current_hierarchy_level = hierarchy_level.clone();
        hierarchy_level.clear();
        for parent_id in current_hierarchy_level {
            for _ in 0..width {
                let child_id = world
                    .spawn(black_box(C::default()))
                    .set_parent(parent_id)
                    .id();
                hierarchy_level.push(child_id)
            }
        }
    }
    world.flush();

    b.iter(move || {
        world.commands().clone_entity_with(id, |builder| {
            builder.recursive(true);
        });
        world.flush();
    });
}

fn simple<C: Bundle + Default + GetTypeRegistration>(b: &mut Bencher) {
    let mut world = World::default();
    let registry = AppTypeRegistry::default();
    {
        let mut r = registry.write();
        r.register::<C>();
    }
    world.insert_resource(registry);
    let id = world.spawn(black_box(C::default())).id();

    b.iter(move || {
        world.commands().clone_entity(id);
        world.flush();
    });
}

fn reflect_benches(c: &mut Criterion) {
    c.bench_function("many components reflect", |b| {
        simple::<(
            ReflectComponent1,
            ReflectComponent2,
            ReflectComponent3,
            ReflectComponent4,
            ReflectComponent5,
            ReflectComponent6,
            ReflectComponent7,
            ReflectComponent8,
            ReflectComponent9,
            ReflectComponent10,
        )>(b);
    });

    c.bench_function("hierarchy wide reflect", |b| {
        hierarchy::<ReflectComponent1>(b, 5, 4);
    });

    c.bench_function("hierarchy tall reflect", |b| {
        hierarchy::<ReflectComponent1>(b, 1, 50);
    });
}

fn clone_benches(c: &mut Criterion) {
    c.bench_function("many components clone", |b| {
        simple::<(
            CloneComponent1,
            CloneComponent2,
            CloneComponent3,
            CloneComponent4,
            CloneComponent5,
            CloneComponent6,
            CloneComponent7,
            CloneComponent8,
            CloneComponent9,
            CloneComponent10,
        )>(b);
    });

    c.bench_function("hierarchy wide clone", |b| {
        hierarchy::<CloneComponent1>(b, 5, 4);
    });

    c.bench_function("hierarchy tall clone", |b| {
        hierarchy::<CloneComponent1>(b, 1, 50);
    });
}
