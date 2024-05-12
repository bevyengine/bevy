use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn query_set(mut queries: ParamSet<(Query<&mut A>, Query<&A>)>) {
    let mut q2 = queries.p0();
    let mut iter2 = q2.iter_mut();
    let mut b = iter2.next().unwrap();

    let q1 = queries.p1();
    //~^ E0499
    let mut iter = q1.iter();
    let a = &*iter.next().unwrap();

    b.0 = a.0
}

fn query_set_flip(mut queries: ParamSet<(Query<&mut A>, Query<&A>)>) {
    let q1 = queries.p1();
    let mut iter = q1.iter();
    let a = &*iter.next().unwrap();

    let mut q2 = queries.p0();
    //~^ E0499
    let mut iter2 = q2.iter_mut();
    let mut b = iter2.next().unwrap();

    b.0 = a.0;
}
