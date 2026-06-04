//! Tests for [`bevy_ptr::MovingPtr`] with panic behavior.

#![allow(unsafe_code, reason = "unsafe is needed to use bevy_ptr::MovingPtr")]

use bevy_ptr::MovingPtr;

use core::cell::Cell;
use core::mem::MaybeUninit;
use core::panic::AssertUnwindSafe;

use std::panic::catch_unwind;

#[test]
fn moving_ptr_assign_drop_is_unwind_safe() {
    struct IncAndPanicOnDrop<'a>(&'a Cell<u32>);
    impl<'a> Drop for IncAndPanicOnDrop<'a> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);

            // Panic, but avoid double-panics to avoid aborts.
            if !std::thread::panicking() {
                panic!();
            }
        }
    }

    let drops1 = Cell::new(0);
    let drops2 = Cell::new(0);

    let mut value1 = MaybeUninit::new(IncAndPanicOnDrop(&drops1));
    let mut value2 = IncAndPanicOnDrop(&drops2);

    _ = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY:
        // - value1 is initialized
        // - we're not using value1 after this point.
        let moving_ptr = unsafe { MovingPtr::from_value(&mut value1) };

        // This should drop value2 and overwrite it with value1 no matter what happen.
        // If the overwrite doesn't happen then it is unsound and the second pair of asserts will fail.
        moving_ptr.assign_to(&mut value2);
    }));

    assert_eq!(drops1.get(), 0);
    assert_eq!(drops2.get(), 1);

    // Now drop value2, which should now hold value1. We expect this to drop value1 and increase drops1.
    _ = catch_unwind(AssertUnwindSafe(|| drop(value2)));

    assert_eq!(drops1.get(), 1);
    assert_eq!(drops2.get(), 1);
}
