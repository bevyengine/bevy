// pathfinder/content/src/transform.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utilities for transforming paths.

use crate::segment::Segment;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::transform3d::Perspective;

/// Transforms a path with a SIMD 2D transform.
pub struct Transform2FPathIter<I>
where
    I: Iterator<Item = Segment>,
{
    iter: I,
    transform: Transform2F,
}

impl<I> Iterator for Transform2FPathIter<I>
where
    I: Iterator<Item = Segment>,
{
    type Item = Segment;

    #[inline]
    fn next(&mut self) -> Option<Segment> {
        // TODO(pcwalton): Can we go faster by transforming an entire line segment with SIMD?
        let mut segment = self.iter.next()?;
        if !segment.is_none() {
            segment.baseline.set_from(self.transform * segment.baseline.from());
            segment.baseline.set_to(self.transform * segment.baseline.to());
            if !segment.is_line() {
                segment.ctrl.set_from(self.transform * segment.ctrl.from());
                if !segment.is_quadratic() {
                    segment.ctrl.set_to(self.transform * segment.ctrl.to());
                }
            }
        }
        Some(segment)
    }
}

impl<I> Transform2FPathIter<I>
where
    I: Iterator<Item = Segment>,
{
    #[inline]
    pub fn new(iter: I, transform: &Transform2F) -> Transform2FPathIter<I> {
        Transform2FPathIter {
            iter,
            transform: *transform,
        }
    }
}

/// Transforms a path with a perspective projection.
pub struct PerspectivePathIter<I>
where
    I: Iterator<Item = Segment>,
{
    iter: I,
    perspective: Perspective,
}

impl<I> Iterator for PerspectivePathIter<I>
where
    I: Iterator<Item = Segment>,
{
    type Item = Segment;

    #[inline]
    fn next(&mut self) -> Option<Segment> {
        let mut segment = self.iter.next()?;
        if !segment.is_none() {
            segment.baseline.set_from(self.perspective * segment.baseline.from());
            segment.baseline.set_to(self.perspective * segment.baseline.to());
            if !segment.is_line() {
                segment.ctrl.set_from(self.perspective * segment.ctrl.from());
                if !segment.is_quadratic() {
                    segment.ctrl.set_to(self.perspective * segment.ctrl.to());
                }
            }
        }
        Some(segment)
    }
}

impl<I> PerspectivePathIter<I>
where
    I: Iterator<Item = Segment>,
{
    #[inline]
    pub fn new(iter: I, perspective: &Perspective) -> PerspectivePathIter<I> {
        PerspectivePathIter {
            iter,
            perspective: *perspective,
        }
    }
}
