use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn query_set(mut queries: QuerySet<(QueryState<&mut A>, QueryState<&A>)>) {
    let mut q2 = queries.q0();
    let mut iter2 = q2.iter_mut();
    let mut b = iter2.next().unwrap();

    let q1 = queries.q1();
    let mut iter = q1.iter();
    let a = &*iter.next().unwrap();

    // this should fail to compile
    b.0 = a.0
}

fn query_set_flip(mut queries: QuerySet<(QueryState<&mut A>, QueryState<&A>)>) {
    let q1 = queries.q1();
    let mut iter = q1.iter();
    let a = &*iter.next().unwrap();

    let mut q2 = queries.q0();
    let mut iter2 = q2.iter_mut();
    let mut b = iter2.next().unwrap();

    // this should fail to compile
    b.0 = a.0;
}

fn main() {}
