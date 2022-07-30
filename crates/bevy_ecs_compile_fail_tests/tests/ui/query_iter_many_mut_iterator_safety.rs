use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Res<Entity>) {
    let iter = query.iter_many_mut([*e]);

    // This should fail to compile.
    is_iterator(iter)
}

fn is_iterator<T: Iterator>(_iter: T) {}

fn main() {}
