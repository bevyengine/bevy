use bevy_ptr::{deconstruct_moving_ptr, MovingPtr};

pub struct A {
    x: usize,
}

#[repr(packed)]
pub struct B {
    x: usize,
}

fn foo(a: MovingPtr<A>, b: MovingPtr<B>) {
    // Moving the same field twice would cause mutable aliased pointers
    deconstruct_moving_ptr!(a => { x, x });
    //~^ E0499
    // A pointer to a member of a `repr(packed)` struct may not be aligned
    deconstruct_moving_ptr!(b => { x });
    //~^ E0793
}

fn bar(a: &mut MovingPtr<A>) {
    // Make sure it only works by value, not by reference,
    // since we may move out of the returned pointers
    deconstruct_moving_ptr!(a => { x });
    //~^ E0308
}
