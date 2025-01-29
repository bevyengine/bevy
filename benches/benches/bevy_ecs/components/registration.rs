use core::hint::black_box;

use bevy_ecs::component::{require, Component, Components, ComponentsData, ComponentsView};
use criterion::Criterion;

#[derive(Component, Default)]
struct ComponentA0;

#[derive(Component, Default)]
struct ComponentA1;

#[derive(Component, Default)]
struct ComponentA2;

#[derive(Component, Default)]
struct ComponentA3;

#[derive(Component, Default)]
struct ComponentA4;

#[derive(Component, Default)]
struct ComponentA5;

#[derive(Component, Default)]
struct ComponentA6;

#[derive(Component, Default)]
struct ComponentA7;

#[derive(Component, Default)]
struct ComponentA8;

#[derive(Component, Default)]
struct ComponentA9;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA2)]
struct ComponentB0;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA3, ComponentA7)]
struct ComponentB1;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA2)]
struct ComponentB2;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA1)]
struct ComponentB3;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA1)]
struct ComponentB4;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA1, ComponentA8, ComponentA4)]
struct ComponentB5;

#[derive(Component, Default)]
#[require(ComponentA0, ComponentA1, ComponentA6)]
struct ComponentB6;

#[derive(Component, Default)]
struct ComponentB7;

#[derive(Component, Default)]
struct ComponentB8;

#[derive(Component, Default)]
struct ComponentB9;

#[derive(Component, Default)]
struct ComponentC0;

#[derive(Component, Default)]
struct ComponentC1;

#[derive(Component, Default)]
#[require(ComponentB1)]
struct ComponentC2;

#[derive(Component, Default)]
#[require(ComponentB1)]
struct ComponentC3;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB2, ComponentB3, ComponentB4)]
struct ComponentC4;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB2, ComponentB4)]
struct ComponentC5;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB9, ComponentB3)]
struct ComponentC6;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB8, ComponentB4)]
struct ComponentC7;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB2, ComponentB6)]
struct ComponentC8;

#[derive(Component, Default)]
#[require(ComponentB1, ComponentB5, ComponentB8)]
struct ComponentC9;

fn register_direct(components: &mut impl ComponentsView) {
    components.register_component::<ComponentA0>();
    components.register_component::<ComponentA1>();
    components.register_component::<ComponentA2>();
    components.register_component::<ComponentA3>();
    components.register_component::<ComponentA4>();
    components.register_component::<ComponentA5>();
    components.register_component::<ComponentA6>();
    components.register_component::<ComponentA7>();
    components.register_component::<ComponentA8>();
    components.register_component::<ComponentA9>();
    components.register_component::<ComponentB0>();
    components.register_component::<ComponentB1>();
    components.register_component::<ComponentB2>();
    components.register_component::<ComponentB3>();
    components.register_component::<ComponentB4>();
    components.register_component::<ComponentB5>();
    components.register_component::<ComponentB6>();
    components.register_component::<ComponentB7>();
    components.register_component::<ComponentB8>();
    components.register_component::<ComponentB9>();
    components.register_component::<ComponentC0>();
    components.register_component::<ComponentC1>();
    components.register_component::<ComponentC2>();
    components.register_component::<ComponentC3>();
    components.register_component::<ComponentC4>();
    components.register_component::<ComponentC5>();
    components.register_component::<ComponentC6>();
    components.register_component::<ComponentC7>();
    components.register_component::<ComponentC8>();
    components.register_component::<ComponentC9>();
}

fn register_synced(components: &Components) {
    components.register_component_synced::<ComponentA0>();
    components.register_component_synced::<ComponentA1>();
    components.register_component_synced::<ComponentA2>();
    components.register_component_synced::<ComponentA3>();
    components.register_component_synced::<ComponentA4>();
    components.register_component_synced::<ComponentA5>();
    components.register_component_synced::<ComponentA6>();
    components.register_component_synced::<ComponentA7>();
    components.register_component_synced::<ComponentA8>();
    components.register_component_synced::<ComponentA9>();
    components.register_component_synced::<ComponentB0>();
    components.register_component_synced::<ComponentB1>();
    components.register_component_synced::<ComponentB2>();
    components.register_component_synced::<ComponentB3>();
    components.register_component_synced::<ComponentB4>();
    components.register_component_synced::<ComponentB5>();
    components.register_component_synced::<ComponentB6>();
    components.register_component_synced::<ComponentB7>();
    components.register_component_synced::<ComponentB8>();
    components.register_component_synced::<ComponentB9>();
    components.register_component_synced::<ComponentC0>();
    components.register_component_synced::<ComponentC1>();
    components.register_component_synced::<ComponentC2>();
    components.register_component_synced::<ComponentC3>();
    components.register_component_synced::<ComponentC4>();
    components.register_component_synced::<ComponentC5>();
    components.register_component_synced::<ComponentC6>();
    components.register_component_synced::<ComponentC7>();
    components.register_component_synced::<ComponentC8>();
    components.register_component_synced::<ComponentC9>();
}

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("registration");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("raw", |b| {
        b.iter(move || {
            let mut components = black_box(ComponentsData::default());
            register_direct(&mut components);
        });
    });
    group.bench_function("mut", |b| {
        b.iter(move || {
            let mut components = black_box(Components::default());
            register_direct(&mut components.as_mut());
        });
    });
    group.bench_function("locked", |b| {
        b.iter(move || {
            let components = black_box(Components::default());
            register_direct(&mut components.lock());
        });
    });
    group.bench_function("full", |b| {
        b.iter(move || {
            let mut components = black_box(Components::default());
            register_direct(&mut components);
        });
    });
    group.bench_function("synced", |b| {
        b.iter(move || {
            let components = black_box(Components::default());
            register_synced(&components);
        });
    });
    group.finish();
}
