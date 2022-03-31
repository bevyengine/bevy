use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>, e: Res<Entity>) {
    let a1 = query.get_many([*e, *e]).unwrap();
    let a2 = query.get_mut(*e).unwrap();
    // this should fail to compile
    println!("{} {}", a1[0].0, a2.0);
}

fn main() {}
