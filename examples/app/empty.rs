//! An empty application (does nothing)

use bevy::prelude::*;

#[derive(Component)]
struct A(f32);

#[derive(Component)]
struct B(f32);

#[no_mangle]
fn test(mut query: Query<(&mut A, &B)>) {
    query.iter_mut().for_each(|(mut a, b)| {
        a.0 += b.0;
    });
}

fn main() {}
