use bevy_ecs::prelude::*;
use std::cmp::Ordering;

#[derive(Component)]
struct A(usize);

fn system(mut query: Query<&mut A>) {
    let iter = query.iter_mut();
    let mut stored: Option<&A> = None;
    let mut sorted = iter.sort_by::<&A>(|left, _right| {
        // Try to smuggle the lens item out of the closure.
        stored = Some(left);
        //~^ E0521
        Ordering::Equal
    });
    let r: &A = stored.unwrap();
    let m: &mut A = &mut sorted.next().unwrap();
    assert!(std::ptr::eq(r, m));
}
