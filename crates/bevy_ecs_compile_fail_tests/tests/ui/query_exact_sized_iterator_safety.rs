use bevy_ecs::prelude::*;

#[derive(Component)]
struct Foo;

fn on_changed(query: Query<&Foo, Changed<Foo>>) {
    // this should fail to compile
    is_exact_size_iterator(query.iter());
}

fn on_added(query: Query<&Foo, Added<Foo>>) {
    // this should fail to compile
    is_exact_size_iterator(query.iter());
}

fn is_exact_size_iterator<T: ExactSizeIterator>(_iter: T) {}

fn main() {}