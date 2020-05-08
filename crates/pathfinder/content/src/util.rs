// pathfinder/content/src/util.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Miscellaneous utilities.

use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_simd::default::{F32x2, F32x4};
use std::hash::{Hash, Hasher};
use std::mem;

pub(crate) fn hash_line_segment<H>(line_segment: LineSegment2F, state: &mut H) where H: Hasher {
    hash_f32x4(line_segment.0, state);
}

pub(crate) fn hash_transform2f<H>(transform: Transform2F, state: &mut H) where H: Hasher {
    hash_f32x4(transform.matrix.0, state);
    hash_f32x2(transform.vector.0, state);
}

pub(crate) fn hash_f32<H>(value: f32, state: &mut H) where H: Hasher {
    unsafe {
        let data: u32 = mem::transmute::<f32, u32>(value);
        data.hash(state);
    }
}

pub(crate) fn hash_f32x2<H>(vector: F32x2, state: &mut H) where H: Hasher {
    unsafe {
        let data: [u32; 2] = mem::transmute::<F32x2, [u32; 2]>(vector);
        data.hash(state);
    }
}

pub(crate) fn hash_f32x4<H>(vector: F32x4, state: &mut H) where H: Hasher {
    unsafe {
        let data: [u32; 4] = mem::transmute::<F32x4, [u32; 4]>(vector);
        data.hash(state);
    }
}
