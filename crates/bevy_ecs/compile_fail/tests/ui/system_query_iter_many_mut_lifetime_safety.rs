use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Entity) {
    let mut results = Vec::new();
    let mut iter = query.iter_many_mut([e, e]);
    //~v E0499
    while let Some(a) = iter.fetch_next() {
        results.push(a);
    }
}
