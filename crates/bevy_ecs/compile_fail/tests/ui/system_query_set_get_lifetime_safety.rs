use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn query_set(mut queries: ParamSet<(Query<&mut A>, Query<&A>)>, e: Entity) {
    let mut q2 = queries.p0();
    let mut b = q2.get_mut(e).unwrap();

    let q1 = queries.p1();
    //~^ E0499
    let a = q1.get(e).unwrap();

    // this should fail to compile
    b.0 = a.0
}

fn query_set_flip(mut queries: ParamSet<(Query<&mut A>, Query<&A>)>, e: Entity) {
    let q1 = queries.p1();
    let a = q1.get(e).unwrap();

    let mut q2 = queries.p0();
    //~^ E0499
    let mut b = q2.get_mut(e).unwrap();

    // this should fail to compile
    b.0 = a.0
}
