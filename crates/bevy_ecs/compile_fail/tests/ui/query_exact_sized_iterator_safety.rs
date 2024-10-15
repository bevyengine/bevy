use bevy_ecs::prelude::*;

#[derive(Component)]
struct Foo;

fn on_changed(query: Query<'_, '_, &Foo, Changed<Foo>>) {
    is_exact_size_iterator(query.iter());
    //~^ E0277
}

fn on_added(query: Query<'_, '_, &Foo, Added<Foo>>) {
    is_exact_size_iterator(query.iter());
    //~^ E0277
}

fn is_exact_size_iterator<T: ExactSizeIterator>(_iter: T) {}
