use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Res<Entity>) {
    let iter = query.iter_combinations_mut();
    // This should fail to compile.
    is_iterator(iter);

    let iter = query.iter_many_mut([*e]);
    // This should fail to compile.
    is_iterator(iter);

    let iter = query.iter_join_map_mut([*e], |e| *e);
    // This should fail to compile.
    is_iterator(iter);
}

fn is_iterator(_iter: impl Iterator) {}

fn main() {}
