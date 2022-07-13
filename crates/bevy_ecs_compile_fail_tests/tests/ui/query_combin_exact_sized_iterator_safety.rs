use bevy_ecs::prelude::*;

#[derive(Component)]
struct Foo;
#[derive(Component)]
struct Bar;

fn on_changed(query: Query<&Foo, Or<(Changed<Foo>, With<Bar>)>>) {
    // this should fail to compile
    is_exact_size_iterator(query.iter_combinations::<2>());
}

fn on_added(query: Query<&Foo, (Added<Foo>, Without<Bar>)>) {
    // this should fail to compile
    is_exact_size_iterator(query.iter_combinations::<2>());
}

fn is_exact_size_iterator<T: ExactSizeIterator>(_iter: T) {}

fn main() {}
