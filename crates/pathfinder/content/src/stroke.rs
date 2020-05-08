// pathfinder/content/src/stroke.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Utilities for converting path strokes to fills.

use crate::outline::{ArcDirection, Contour, ContourIterFlags, Outline, PushSegmentFlags};
use crate::segment::Segment;
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::util::EPSILON;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use std::f32;

const TOLERANCE: f32 = 0.01;

pub struct OutlineStrokeToFill<'a> {
    input: &'a Outline,
    output: Outline,
    style: StrokeStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeStyle {
    pub line_width: f32,
    pub line_cap: LineCap,
    pub line_join: LineJoin,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineCap {
    Butt,
    Square,
    Round,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineJoin {
    Miter(f32),
    Bevel,
    Round,
}

impl<'a> OutlineStrokeToFill<'a> {
    #[inline]
    pub fn new(input: &Outline, style: StrokeStyle) -> OutlineStrokeToFill {
        OutlineStrokeToFill { input, output: Outline::new(), style }
    }

    pub fn offset(&mut self) {
        let mut new_contours = vec![];
        for input in &self.input.contours {
            let closed = input.closed;
            let mut stroker = ContourStrokeToFill::new(input,
                                                       Contour::new(),
                                                       self.style.line_width * 0.5,
                                                       self.style.line_join);

            stroker.offset_forward();
            if closed {
                self.push_stroked_contour(&mut new_contours, stroker, true);
                stroker = ContourStrokeToFill::new(input,
                                                   Contour::new(),
                                                   self.style.line_width * 0.5,
                                                   self.style.line_join);
            } else {
                self.add_cap(&mut stroker.output);
            }

            stroker.offset_backward();
            if !closed {
                self.add_cap(&mut stroker.output);
            }

            self.push_stroked_contour(&mut new_contours, stroker, closed);
        }

        let mut new_bounds = None;
        new_contours.iter().for_each(|contour| contour.update_bounds(&mut new_bounds));

        self.output.contours = new_contours;
        self.output.bounds = new_bounds.unwrap_or_else(|| RectF::default());
    }

    #[inline]
    pub fn into_outline(self) -> Outline {
        self.output
    }

    fn push_stroked_contour(&mut self,
                            new_contours: &mut Vec<Contour>,
                            mut stroker: ContourStrokeToFill,
                            closed: bool) {
        // Add join if necessary.
        if closed && stroker.output.might_need_join(self.style.line_join) {
            let (p1, p0) = (stroker.output.position_of(1), stroker.output.position_of(0));
            let final_segment = LineSegment2F::new(p1, p0);
            stroker.output.add_join(self.style.line_width * 0.5,
                                    self.style.line_join,
                                    stroker.input.position_of(0),
                                    final_segment);
        }

        stroker.output.closed = true;
        new_contours.push(stroker.output);
    }

    fn add_cap(&mut self, contour: &mut Contour) {
        if self.style.line_cap == LineCap::Butt || contour.len() < 2 {
            return
        }

        let width = self.style.line_width;
        let p1 = contour.position_of_last(1);

        // Determine the ending gradient.
        let mut p0;
        let mut p0_index = contour.len() - 2;
        loop {
            p0 = contour.position_of(p0_index);
            if (p1 - p0).square_length() > EPSILON {
                break;
            }
            if p0_index == 0 {
                return;
            }
            p0_index -= 1;
        }
        let gradient = (p1 - p0).normalize();

        match self.style.line_cap {
            LineCap::Butt => unreachable!(),

            LineCap::Square => {
                let offset = gradient * (width * 0.5);

                let p2 = p1 + offset;
                let p3 = p2 + gradient.yx() * vec2f(-width, width);
                let p4 = p3 - offset;

                contour.push_endpoint(p2);
                contour.push_endpoint(p3);
                contour.push_endpoint(p4);
            }

            LineCap::Round => {
                let scale = width * 0.5;
                let offset = gradient.yx() * vec2f(-1.0, 1.0);
                let translation = p1 + offset * (width * 0.5);
                let transform = Transform2F::from_scale(scale).translate(translation);
                let chord = LineSegment2F::new(-offset, offset);
                contour.push_arc_from_unit_chord(&transform, chord, ArcDirection::CW);
            }
        }
    }
}

struct ContourStrokeToFill<'a> {
    input: &'a Contour,
    output: Contour,
    radius: f32,
    join: LineJoin,
}

impl<'a> ContourStrokeToFill<'a> {
    #[inline]
    fn new(input: &Contour, output: Contour, radius: f32, join: LineJoin) -> ContourStrokeToFill {
        ContourStrokeToFill { input, output, radius, join }
    }

    fn offset_forward(&mut self) {
        for (segment_index, segment) in self.input.iter(ContourIterFlags::empty()).enumerate() {
            // FIXME(pcwalton): We negate the radius here so that round end caps can be drawn
            // clockwise. Of course, we should just implement anticlockwise arcs to begin with...
            let join = if segment_index == 0 { LineJoin::Bevel } else { self.join };
            segment.offset(-self.radius, join, &mut self.output);
        }
    }

    fn offset_backward(&mut self) {
        let mut segments: Vec<_> = self
            .input
            .iter(ContourIterFlags::empty())
            .map(|segment| segment.reversed())
            .collect();
        segments.reverse();
        for (segment_index, segment) in segments.iter().enumerate() {
            // FIXME(pcwalton): We negate the radius here so that round end caps can be drawn
            // clockwise. Of course, we should just implement anticlockwise arcs to begin with...
            let join = if segment_index == 0 { LineJoin::Bevel } else { self.join };
            segment.offset(-self.radius, join, &mut self.output);
        }
    }
}

trait Offset {
    fn offset(&self, distance: f32, join: LineJoin, contour: &mut Contour);
    fn add_to_contour(&self,
                      distance: f32,
                      join: LineJoin,
                      join_point: Vector2F,
                      contour: &mut Contour);
    fn offset_once(&self, distance: f32) -> Self;
    fn error_is_within_tolerance(&self, other: &Segment, distance: f32) -> bool;
}

impl Offset for Segment {
    fn offset(&self, distance: f32, join: LineJoin, contour: &mut Contour) {
        let join_point = self.baseline.from();
        if self.baseline.square_length() < TOLERANCE * TOLERANCE {
            self.add_to_contour(distance, join, join_point, contour);
            return;
        }

        let candidate = self.offset_once(distance);
        if self.error_is_within_tolerance(&candidate, distance) {
            candidate.add_to_contour(distance, join, join_point, contour);
            return;
        }

        debug!("--- SPLITTING ---");
        debug!("... PRE-SPLIT: {:?}", self);
        let (before, after) = self.split(0.5);
        debug!("... AFTER-SPLIT: {:?} {:?}", before, after);
        before.offset(distance, join, contour);
        after.offset(distance, join, contour);
    }

    fn add_to_contour(&self,
                      distance: f32,
                      join: LineJoin,
                      join_point: Vector2F,
                      contour: &mut Contour) {
        // Add join if necessary.
        if contour.might_need_join(join) {
            let p3 = self.baseline.from();
            let p4 = if self.is_line() {
                self.baseline.to()
            } else {
                // NB: If you change the representation of quadratic curves, you will need to
                // change this.
                self.ctrl.from()
            };

            contour.add_join(distance, join, join_point, LineSegment2F::new(p4, p3));
        }

        // Push segment.
        let flags = PushSegmentFlags::UPDATE_BOUNDS | PushSegmentFlags::INCLUDE_FROM_POINT;
        contour.push_segment(self, flags);
    }

    fn offset_once(&self, distance: f32) -> Segment {
        if self.is_line() {
            return Segment::line(self.baseline.offset(distance));
        }

        if self.is_quadratic() {
            let mut segment_0 = LineSegment2F::new(self.baseline.from(), self.ctrl.from());
            let mut segment_1 = LineSegment2F::new(self.ctrl.from(), self.baseline.to());
            segment_0 = segment_0.offset(distance);
            segment_1 = segment_1.offset(distance);
            let ctrl = match segment_0.intersection_t(segment_1) {
                Some(t) => segment_0.sample(t),
                None => segment_0.to().lerp(segment_1.from(), 0.5),
            };
            let baseline = LineSegment2F::new(segment_0.from(), segment_1.to());
            return Segment::quadratic(baseline, ctrl);
        }

        debug_assert!(self.is_cubic());

        if self.baseline.from() == self.ctrl.from() {
            let mut segment_0 = LineSegment2F::new(self.baseline.from(), self.ctrl.to());
            let mut segment_1 = LineSegment2F::new(self.ctrl.to(), self.baseline.to());
            segment_0 = segment_0.offset(distance);
            segment_1 = segment_1.offset(distance);
            let ctrl = match segment_0.intersection_t(segment_1) {
                Some(t) => segment_0.sample(t),
                None => segment_0.to().lerp(segment_1.from(), 0.5),
            };
            let baseline = LineSegment2F::new(segment_0.from(), segment_1.to());
            let ctrl = LineSegment2F::new(segment_0.from(), ctrl);
            return Segment::cubic(baseline, ctrl);
        }

        if self.ctrl.to() == self.baseline.to() {
            let mut segment_0 = LineSegment2F::new(self.baseline.from(), self.ctrl.from());
            let mut segment_1 = LineSegment2F::new(self.ctrl.from(), self.baseline.to());
            segment_0 = segment_0.offset(distance);
            segment_1 = segment_1.offset(distance);
            let ctrl = match segment_0.intersection_t(segment_1) {
                Some(t) => segment_0.sample(t),
                None => segment_0.to().lerp(segment_1.from(), 0.5),
            };
            let baseline = LineSegment2F::new(segment_0.from(), segment_1.to());
            let ctrl = LineSegment2F::new(ctrl, segment_1.to());
            return Segment::cubic(baseline, ctrl);
        }

        let mut segment_0 = LineSegment2F::new(self.baseline.from(), self.ctrl.from());
        let mut segment_1 = LineSegment2F::new(self.ctrl.from(), self.ctrl.to());
        let mut segment_2 = LineSegment2F::new(self.ctrl.to(), self.baseline.to());
        segment_0 = segment_0.offset(distance);
        segment_1 = segment_1.offset(distance);
        segment_2 = segment_2.offset(distance);
        let (ctrl_0, ctrl_1) = match (
            segment_0.intersection_t(segment_1),
            segment_1.intersection_t(segment_2),
        ) {
            (Some(t0), Some(t1)) => (segment_0.sample(t0), segment_1.sample(t1)),
            _ => (
                segment_0.to().lerp(segment_1.from(), 0.5),
                segment_1.to().lerp(segment_2.from(), 0.5),
            ),
        };
        let baseline = LineSegment2F::new(segment_0.from(), segment_2.to());
        let ctrl = LineSegment2F::new(ctrl_0, ctrl_1);
        Segment::cubic(baseline, ctrl)
    }

    fn error_is_within_tolerance(&self, other: &Segment, distance: f32) -> bool {
        let (mut min, mut max) = (
            f32::abs(distance) - TOLERANCE,
            f32::abs(distance) + TOLERANCE,
        );
        min = if min <= 0.0 { 0.0 } else { min * min };
        max = if max <= 0.0 { 0.0 } else { max * max };

        for t_num in 0..(SAMPLE_COUNT + 1) {
            let t = t_num as f32 / SAMPLE_COUNT as f32;
            // FIXME(pcwalton): Use signed distance!
            let (this_p, other_p) = (self.sample(t), other.sample(t));
            let vector = this_p - other_p;
            let square_distance = vector.square_length();
            debug!(
                "this_p={:?} other_p={:?} vector={:?} sqdist={:?} min={:?} max={:?}",
                this_p, other_p, vector, square_distance, min, max
            );
            if square_distance < min || square_distance > max {
                return false;
            }
        }

        return true;

        const SAMPLE_COUNT: u32 = 16;
    }
}

impl Contour {
    fn might_need_join(&self, join: LineJoin) -> bool {
        if self.len() < 2 {
            false
        } else {
            match join {
                LineJoin::Miter(_) | LineJoin::Round => true,
                LineJoin::Bevel => false,
            }
        }
    }

    fn add_join(&mut self,
                distance: f32,
                join: LineJoin,
                join_point: Vector2F,
                next_tangent: LineSegment2F) {
        let (p0, p1) = (self.position_of_last(2), self.position_of_last(1));
        let prev_tangent = LineSegment2F::new(p0, p1);

        if prev_tangent.square_length() < EPSILON || next_tangent.square_length() < EPSILON {
            return;
        }

        match join {
            LineJoin::Bevel => {}
            LineJoin::Miter(miter_limit) => {
                if let Some(prev_tangent_t) = prev_tangent.intersection_t(next_tangent) {
                    if prev_tangent_t < -EPSILON {
                        return;
                    }
                    let miter_endpoint = prev_tangent.sample(prev_tangent_t);
                    let threshold = miter_limit * distance;
                    if (miter_endpoint - join_point).square_length() > threshold * threshold {
                        return;
                    }
                    self.push_endpoint(miter_endpoint);
                }
            }
            LineJoin::Round => {
                let scale = distance.abs();
                let transform = Transform2F::from_scale(scale).translate(join_point);
                let chord_from = (prev_tangent.to() - join_point).normalize();
                let chord_to = (next_tangent.to() - join_point).normalize();
                let chord = LineSegment2F::new(chord_from, chord_to);
                self.push_arc_from_unit_chord(&transform, chord, ArcDirection::CW);
            }
        }
    }
}

impl Default for StrokeStyle {
    #[inline]
    fn default() -> StrokeStyle {
        StrokeStyle {
            line_width: 1.0,
            line_cap: LineCap::default(),
            line_join: LineJoin::default(),
        }
    }
}

impl Default for LineCap {
    #[inline]
    fn default() -> LineCap { LineCap::Butt }
}

impl Default for LineJoin {
    #[inline]
    fn default() -> LineJoin { LineJoin::Miter(10.0) }
}
