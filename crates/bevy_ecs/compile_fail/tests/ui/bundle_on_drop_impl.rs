use bevy_ecs::prelude::*;

#[derive(Component, Debug)]
pub struct A(usize);

// this should fail since destructuring T: Drop cannot be split.
#[derive(Bundle, Debug)]
//~^ E0509
pub struct DropBundle {
    component_a: A,
}

impl Drop for DropBundle {
    fn drop(&mut self) {
        // Just need the impl
    }
}
