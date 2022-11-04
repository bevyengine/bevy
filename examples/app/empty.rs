//! An empty application (does nothing)

use bevy::prelude::*;

#[derive(Component)]
pub struct TestA(f32);

#[derive(Component)]
pub struct TestB(f32);

#[no_mangle]
fn iter(mut query: Query<(&mut TestA, &TestB)>) {
    for (mut a, b) in query.iter_mut() {
        a.0 += b.0;
    }
}

fn main() {}
