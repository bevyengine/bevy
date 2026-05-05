use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Entity) {
    let iter = query.iter_many_mut([e]);

    is_iterator(iter)
    //~^ E0277
}

fn is_iterator<T: Iterator>(_iter: T) {}
