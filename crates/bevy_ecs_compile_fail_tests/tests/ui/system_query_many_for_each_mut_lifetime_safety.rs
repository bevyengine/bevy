use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Res<Entity>) {
    let mut results = Vec::new();
    query.many_for_each_mut(vec![*e, *e], |a| {
        // this should fail to compile
        results.push(a);
    });
}

fn main() {}
