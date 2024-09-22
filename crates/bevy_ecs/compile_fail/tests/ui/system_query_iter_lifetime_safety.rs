use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>) {
    let mut iter = query.iter_mut();
    let a = &mut *iter.next().unwrap();

    let mut iter2 = query.iter_mut();
    //~^ E0499
    let _ = &mut *iter2.next().unwrap();

    println!("{}", a.0);
}
