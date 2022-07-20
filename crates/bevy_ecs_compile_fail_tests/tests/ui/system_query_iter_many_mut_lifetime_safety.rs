use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Res<Entity>) {
    let mut results = Vec::new();
    let mut iter = query.iter_many_mut([*e, *e]);
    while let Some(a) = iter.fetch_next() {
        // this should fail to compile
        results.push(a);
    }
}

fn main() {}
