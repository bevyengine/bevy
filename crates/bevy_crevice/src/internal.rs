//! This module is internal to crevice but used by its derive macro. No
//! guarantees are made about its contents.

pub use bytemuck;

/// Gives the number of bytes needed to make `offset` be aligned to `alignment`.
pub const fn align_offset(offset: usize, alignment: usize) -> usize {
    if alignment == 0 || offset % alignment == 0 {
        0
    } else {
        alignment - offset % alignment
    }
}

/// Max of two `usize`. Implemented because the `max` method from `Ord` cannot
/// be used in const fns.
pub const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

/// Max of an array of `usize`. This function's implementation is funky because
/// we have no for loops!
pub const fn max_arr<const N: usize>(input: [usize; N]) -> usize {
    let mut max = 0;
    let mut i = 0;

    while i < N {
        if input[i] > max {
            max = input[i];
        }

        i += 1;
    }

    max
}
