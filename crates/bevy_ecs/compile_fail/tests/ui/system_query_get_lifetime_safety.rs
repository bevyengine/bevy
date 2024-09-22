use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Entity) {
    let a1 = query.get_mut(e).unwrap();
    let a2 = query.get_mut(e).unwrap();
    //~^ E0499
    println!("{} {}", a1.0, a2.0);
}
