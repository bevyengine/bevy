//@no-rustfix

use bevy_ecs::ptr::{deconstruct_moving_ptr, MovingPtr};

pub struct A {
    x: usize,
}

#[repr(packed)]
pub struct B {
    x: usize,
}

fn test1(
    a: MovingPtr<A>,
    box_a: MovingPtr<Box<A>>,
    mut_a: MovingPtr<&mut A>,
    box_t: MovingPtr<Box<(usize, usize)>>,
    mut_t: MovingPtr<&mut (usize, usize)>,
) {
    // Moving the same field twice would cause mutable aliased pointers
    //~v E0025
    deconstruct_moving_ptr!({
        let A { x, x } = a;
    });
    // Field offsets would not be valid through autoderef
    //~vv E0308
    //~v E0308
    deconstruct_moving_ptr!({
        let A { x } = box_a;
    });
    //~v E0308
    deconstruct_moving_ptr!({
        let A { x } = mut_a;
    });
    //~v E0308
    deconstruct_moving_ptr!({
        let tuple { 0: _, 1: _ } = box_t;
    });
    //~v E0308
    deconstruct_moving_ptr!({
        let tuple { 0: _, 1: _ } = mut_t;
    });
}

fn test2(t: MovingPtr<(usize, usize)>) {
    // Moving the same field twice would cause mutable aliased pointers
    //~v E0499
    deconstruct_moving_ptr!({
        let tuple { 0: _, 0: _ } = t;
    });
}

fn test3(b: MovingPtr<B>) {
    // A pointer to a member of a `repr(packed)` struct may not be aligned
    //~v E0793
    deconstruct_moving_ptr!({
        let B { x } = b;
    });
}

fn test4(a: &mut MovingPtr<A>, t: &mut MovingPtr<(usize, usize)>) {
    // Make sure it only takes `MovingPtr` by value and not by reference,
    // since the child `MovingPtr`s will drop part of the parent
    deconstruct_moving_ptr!({
        //~v E0308
        let A { x } = a;
    });
    deconstruct_moving_ptr!({
        //~v E0308
        let tuple { 0: _, 1: _ } = t;
    });
}
