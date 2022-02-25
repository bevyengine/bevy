use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn query_set(mut queries: QuerySet<(QueryState<&mut A>, QueryState<&A>)>, e: Res<Entity>) {
    let mut q2 = queries.q0();
    let mut b = q2.get_mut(*e).unwrap();

    let q1 = queries.q1();
    let a = q1.get(*e).unwrap();

    // this should fail to compile
    b.0 = a.0
}

fn query_set_flip(mut queries: QuerySet<(QueryState<&mut A>, QueryState<&A>)>, e: Res<Entity>) {
    let q1 = queries.q1();
    let a = q1.get(*e).unwrap();

    let mut q2 = queries.q0();
    let mut b = q2.get_mut(*e).unwrap();

    // this should fail to compile
    b.0 = a.0
}

fn main() {}
