// pathfinder/content/src/outline.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A compressed in-memory representation of paths.

use crate::clip::{self, ContourPolygonClipper, ContourRectClipper};
use crate::dilation::ContourDilator;
use crate::orientation::Orientation;
use crate::segment::{Segment, SegmentFlags, SegmentKind};
use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::transform3d::Perspective;
use pathfinder_geometry::unit_vector::UnitVector;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use std::f32::consts::PI;
use std::fmt::{self, Debug, Formatter};
use std::mem;

#[derive(Clone)]
pub struct Outline {
    pub(crate) contours: Vec<Contour>,
    pub(crate) bounds: RectF,
}

#[derive(Clone)]
pub struct Contour {
    pub(crate) points: Vec<Vector2F>,
    pub(crate) flags: Vec<PointFlags>,
    pub(crate) bounds: RectF,
    pub(crate) closed: bool,
}

bitflags! {
    pub struct PointFlags: u8 {
        const CONTROL_POINT_0 = 0x01;
        const CONTROL_POINT_1 = 0x02;
    }
}

bitflags! {
    pub struct PushSegmentFlags: u8 {
        const UPDATE_BOUNDS = 0x01;
        const INCLUDE_FROM_POINT = 0x02;
    }
}

impl Outline {
    #[inline]
    pub fn new() -> Outline {
        Outline {
            contours: vec![],
            bounds: RectF::default(),
        }
    }

    #[inline]
    pub fn from_segments<I>(segments: I) -> Outline
    where
        I: Iterator<Item = Segment>,
    {
        let mut outline = Outline::new();
        let mut current_contour = Contour::new();

        for segment in segments {
            if segment.flags.contains(SegmentFlags::FIRST_IN_SUBPATH) {
                if !current_contour.is_empty() {
                    outline
                        .contours
                        .push(mem::replace(&mut current_contour, Contour::new()));
                }
                current_contour.push_point(segment.baseline.from(), PointFlags::empty(), true);
            }

            if segment.flags.contains(SegmentFlags::CLOSES_SUBPATH) {
                if !current_contour.is_empty() {
                    current_contour.close();
                    let contour = mem::replace(&mut current_contour, Contour::new());
                    outline.push_contour(contour);
                }
                continue;
            }

            if segment.is_none() {
                continue;
            }

            if !segment.is_line() {
                current_contour.push_point(segment.ctrl.from(), PointFlags::CONTROL_POINT_0, true);
                if !segment.is_quadratic() {
                    current_contour.push_point(
                        segment.ctrl.to(),
                        PointFlags::CONTROL_POINT_1,
                        true,
                    );
                }
            }

            current_contour.push_point(segment.baseline.to(), PointFlags::empty(), true);
        }

        outline.push_contour(current_contour);
        outline
    }

    #[inline]
    pub fn from_rect(rect: RectF) -> Outline {
        let mut outline = Outline::new();
        outline.push_contour(Contour::from_rect(rect));
        outline
    }

    #[inline]
    pub fn bounds(&self) -> RectF {
        self.bounds
    }

    #[inline]
    pub fn contours(&self) -> &[Contour] {
        &self.contours
    }

    #[inline]
    pub fn into_contours(self) -> Vec<Contour> {
        self.contours
    }

    /// Removes all contours from this outline.
    #[inline]
    pub fn clear(&mut self) {
        self.contours.clear();
        self.bounds = RectF::default();
    }

    pub fn push_contour(&mut self, contour: Contour) {
        if contour.is_empty() {
            return;
        }

        if self.contours.is_empty() {
            self.bounds = contour.bounds;
        } else {
            self.bounds = self.bounds.union_rect(contour.bounds);
        }

        self.contours.push(contour);
    }

    pub fn pop_contour(&mut self) -> Option<Contour> {
        let last_contour = self.contours.pop();

        let mut new_bounds = None;
        for contour in &mut self.contours {
            contour.update_bounds(&mut new_bounds);
        }
        self.bounds = new_bounds.unwrap_or_else(|| RectF::default());

        last_contour
    }

    pub fn transform(&mut self, transform: &Transform2F) {
        if transform.is_identity() {
            return;
        }

        let mut new_bounds = None;
        for contour in &mut self.contours {
            contour.transform(transform);
            contour.update_bounds(&mut new_bounds);
        }
        self.bounds = new_bounds.unwrap_or_else(|| RectF::default());
    }

    pub fn apply_perspective(&mut self, perspective: &Perspective) {
        let mut new_bounds = None;
        for contour in &mut self.contours {
            contour.apply_perspective(perspective);
            contour.update_bounds(&mut new_bounds);
        }
        self.bounds = new_bounds.unwrap_or_else(|| RectF::default());
    }

    pub fn dilate(&mut self, amount: Vector2F) {
        let orientation = Orientation::from_outline(self);
        self.contours
            .iter_mut()
            .for_each(|contour| contour.dilate(amount, orientation));
        self.bounds = self.bounds.dilate(amount);
    }

    pub fn prepare_for_tiling(&mut self, view_box: RectF) {
        self.contours
            .iter_mut()
            .for_each(|contour| contour.prepare_for_tiling(view_box));
        self.bounds = self
            .bounds
            .intersection(view_box)
            .unwrap_or_else(|| RectF::default());
    }

    pub fn is_outside_polygon(&self, clip_polygon: &[Vector2F]) -> bool {
        clip::rect_is_outside_polygon(self.bounds, clip_polygon)
    }

    fn is_inside_polygon(&self, clip_polygon: &[Vector2F]) -> bool {
        clip::rect_is_inside_polygon(self.bounds, clip_polygon)
    }

    pub fn clip_against_polygon(&mut self, clip_polygon: &[Vector2F]) {
        // Quick check.
        if self.is_inside_polygon(clip_polygon) {
            return;
        }

        for contour in mem::replace(&mut self.contours, vec![]) {
            self.push_contour(ContourPolygonClipper::new(clip_polygon, contour).clip());
        }
    }

    pub fn clip_against_rect(&mut self, clip_rect: RectF) {
        if clip_rect.contains_rect(self.bounds) {
            return;
        }

        for contour in mem::replace(&mut self.contours, vec![]) {
            self.push_contour(ContourRectClipper::new(clip_rect, contour).clip());
        }
    }

    #[inline]
    pub fn close_all_contours(&mut self) {
        self.contours.iter_mut().for_each(|contour| contour.close());
    }
}

impl Debug for Outline {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        for (contour_index, contour) in self.contours.iter().enumerate() {
            if contour_index > 0 {
                write!(formatter, " ")?;
            }
            contour.fmt(formatter)?;
        }
        Ok(())
    }
}

impl Contour {
    #[inline]
    pub fn new() -> Contour {
        Contour {
            points: vec![],
            flags: vec![],
            bounds: RectF::default(),
            closed: false,
        }
    }

    #[inline]
    pub fn with_capacity(length: usize) -> Contour {
        Contour {
            points: Vec::with_capacity(length),
            flags: Vec::with_capacity(length),
            bounds: RectF::default(),
            closed: false,
        }
    }

    #[inline]
    pub fn from_rect(rect: RectF) -> Contour {
        let mut contour = Contour::new();
        contour.push_point(rect.origin(), PointFlags::empty(), false);
        contour.push_point(rect.upper_right(), PointFlags::empty(), false);
        contour.push_point(rect.lower_right(), PointFlags::empty(), false);
        contour.push_point(rect.lower_left(), PointFlags::empty(), false);
        contour.close();
        contour.bounds = rect;
        contour
    }

    // Replaces this contour with a new one, with arrays preallocated to match `self`.
    #[inline]
    pub(crate) fn take(&mut self) -> Contour {
        let length = self.len() as usize;
        mem::replace(
            self,
            Contour {
                points: Vec::with_capacity(length),
                flags: Vec::with_capacity(length),
                bounds: RectF::default(),
                closed: false,
            },
        )
    }

    /// restore self to the state of Contour::new(), but keep the points buffer allocated
    #[inline]
    pub fn clear(&mut self) {
        self.points.clear();
        self.flags.clear();
        self.bounds = RectF::default();
        self.closed = false;
    }

    #[inline]
    pub fn iter(&self, flags: ContourIterFlags) -> ContourIter {
        ContourIter {
            contour: self,
            index: 1,
            flags,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    #[inline]
    pub fn len(&self) -> u32 {
        self.points.len() as u32
    }

    #[inline]
    pub fn bounds(&self) -> RectF {
        self.bounds
    }

    #[inline]
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    #[inline]
    pub fn position_of(&self, index: u32) -> Vector2F {
        self.points[index as usize]
    }

    #[inline]
    pub fn last_position(&self) -> Option<Vector2F> {
        self.points.last().cloned()
    }

    #[inline]
    pub(crate) fn position_of_last(&self, index: u32) -> Vector2F {
        self.points[self.points.len() - index as usize]
    }

    #[inline]
    pub fn push_endpoint(&mut self, point: Vector2F) {
        self.push_point(point, PointFlags::empty(), true);
    }

    #[inline]
    pub fn push_quadratic(&mut self, ctrl: Vector2F, point: Vector2F) {
        self.push_point(ctrl, PointFlags::CONTROL_POINT_0, true);
        self.push_point(point, PointFlags::empty(), true);
    }

    #[inline]
    pub fn push_cubic(&mut self, ctrl0: Vector2F, ctrl1: Vector2F, point: Vector2F) {
        self.push_point(ctrl0, PointFlags::CONTROL_POINT_0, true);
        self.push_point(ctrl1, PointFlags::CONTROL_POINT_1, true);
        self.push_point(point, PointFlags::empty(), true);
    }

    #[inline]
    pub fn close(&mut self) {
        self.closed = true;
    }

    #[inline]
    pub(crate) fn push_point(&mut self,
                             point: Vector2F,
                             flags: PointFlags,
                             update_bounds: bool) {
        debug_assert!(!point.x().is_nan() && !point.y().is_nan());

        if update_bounds {
            let first = self.is_empty();
            union_rect(&mut self.bounds, point, first);
        }

        self.points.push(point);
        self.flags.push(flags);
    }

    #[inline]
    pub(crate) fn push_segment(&mut self, segment: &Segment, flags: PushSegmentFlags) {
        if segment.is_none() {
            return;
        }

        let update_bounds = flags.contains(PushSegmentFlags::UPDATE_BOUNDS);
        self.push_point(segment.baseline.from(), PointFlags::empty(), update_bounds);

        if !segment.is_line() {
            self.push_point(
                segment.ctrl.from(),
                PointFlags::CONTROL_POINT_0,
                update_bounds,
            );
            if !segment.is_quadratic() {
                self.push_point(
                    segment.ctrl.to(),
                    PointFlags::CONTROL_POINT_1,
                    update_bounds,
                );
            }
        }

        self.push_point(segment.baseline.to(), PointFlags::empty(), update_bounds);
    }

    pub fn push_arc(&mut self,
                    transform: &Transform2F,
                    start_angle: f32,
                    end_angle: f32,
                    direction: ArcDirection) {
        if end_angle - start_angle >= PI * 2.0 {
            self.push_ellipse(transform);
        } else {
            let start = vec2f(start_angle.cos(), start_angle.sin());
            let end   = vec2f(end_angle.cos(),   end_angle.sin());
            self.push_arc_from_unit_chord(transform, LineSegment2F::new(start, end), direction);
        }
    }

    pub fn push_arc_from_unit_chord(&mut self,
                                    transform: &Transform2F,
                                    mut chord: LineSegment2F,
                                    direction: ArcDirection) {
        let mut direction_transform = Transform2F::default();
        if direction == ArcDirection::CCW {
            chord *= vec2f(1.0, -1.0);
            direction_transform = Transform2F::from_scale(vec2f(1.0, -1.0));
        }

        let (mut vector, end_vector) = (UnitVector(chord.from()), UnitVector(chord.to()));
        for segment_index in 0..4 {
            debug!("push_arc_from_unit_chord(): loop segment index {}", segment_index);

            let mut sweep_vector = end_vector.rev_rotate_by(vector);
            let last = sweep_vector.0.x() >= -EPSILON && sweep_vector.0.y() >= -EPSILON;
            debug!("... end_vector={:?} vector={:?} sweep_vector={:?} last={:?}",
                   end_vector,
                   vector,
                   sweep_vector,
                   last);

            let mut segment;
            if !last {
                sweep_vector = UnitVector(vec2f(0.0, 1.0));
                segment = Segment::quarter_circle_arc();
            } else {
                segment = Segment::arc_from_cos(sweep_vector.0.x());
            }

            let half_sweep_vector = sweep_vector.halve_angle();
            let rotation = Transform2F::from_rotation_vector(half_sweep_vector.rotate_by(vector));
            segment = segment.transform(&(*transform * direction_transform * rotation));

            let mut push_segment_flags = PushSegmentFlags::UPDATE_BOUNDS;
            if segment_index == 0 {
                push_segment_flags.insert(PushSegmentFlags::INCLUDE_FROM_POINT);
            }
            self.push_segment(&segment, push_segment_flags);

            if last {
                break;
            }

            vector = vector.rotate_by(sweep_vector);
        }

        const EPSILON: f32 = 0.001;
    }

    pub fn push_ellipse(&mut self, transform: &Transform2F) {
        let segment = Segment::quarter_circle_arc();
        let mut rotation;
        self.push_segment(&segment.transform(transform),
                          PushSegmentFlags::UPDATE_BOUNDS | PushSegmentFlags::INCLUDE_FROM_POINT);
        rotation = Transform2F::from_rotation_vector(UnitVector(vec2f( 0.0,  1.0)));
        self.push_segment(&segment.transform(&(*transform * rotation)),
                          PushSegmentFlags::UPDATE_BOUNDS);
        rotation = Transform2F::from_rotation_vector(UnitVector(vec2f(-1.0,  0.0)));
        self.push_segment(&segment.transform(&(*transform * rotation)),
                          PushSegmentFlags::UPDATE_BOUNDS);
        rotation = Transform2F::from_rotation_vector(UnitVector(vec2f( 0.0, -1.0)));
        self.push_segment(&segment.transform(&(*transform * rotation)),
                          PushSegmentFlags::UPDATE_BOUNDS);
    }

    #[inline]
    pub fn segment_after(&self, point_index: u32) -> Segment {
        debug_assert!(self.point_is_endpoint(point_index));

        let mut segment = Segment::none();
        segment.baseline.set_from(self.position_of(point_index));

        let point1_index = self.add_to_point_index(point_index, 1);
        if self.point_is_endpoint(point1_index) {
            segment.baseline.set_to(self.position_of(point1_index));
            segment.kind = SegmentKind::Line;
        } else {
            segment.ctrl.set_from(self.position_of(point1_index));

            let point2_index = self.add_to_point_index(point_index, 2);
            if self.point_is_endpoint(point2_index) {
                segment.baseline.set_to(self.position_of(point2_index));
                segment.kind = SegmentKind::Quadratic;
            } else {
                segment.ctrl.set_to(self.position_of(point2_index));
                segment.kind = SegmentKind::Cubic;

                let point3_index = self.add_to_point_index(point_index, 3);
                segment.baseline.set_to(self.position_of(point3_index));
            }
        }

        segment
    }

    #[inline]
    pub fn hull_segment_after(&self, prev_point_index: u32) -> LineSegment2F {
        let next_point_index = self.next_point_index_of(prev_point_index);
        LineSegment2F::new(
            self.points[prev_point_index as usize],
            self.points[next_point_index as usize],
        )
    }

    #[inline]
    pub fn point_is_endpoint(&self, point_index: u32) -> bool {
        !self.flags[point_index as usize]
            .intersects(PointFlags::CONTROL_POINT_0 | PointFlags::CONTROL_POINT_1)
    }

    #[inline]
    pub fn add_to_point_index(&self, point_index: u32, addend: u32) -> u32 {
        let (index, limit) = (point_index + addend, self.len());
        if index >= limit {
            index - limit
        } else {
            index
        }
    }

    #[inline]
    pub fn point_is_logically_above(&self, a: u32, b: u32) -> bool {
        let (a_y, b_y) = (self.points[a as usize].y(), self.points[b as usize].y());
        a_y < b_y || (a_y == b_y && a < b)
    }

    #[inline]
    pub fn prev_endpoint_index_of(&self, mut point_index: u32) -> u32 {
        loop {
            point_index = self.prev_point_index_of(point_index);
            if self.point_is_endpoint(point_index) {
                return point_index;
            }
        }
    }

    #[inline]
    pub fn next_endpoint_index_of(&self, mut point_index: u32) -> u32 {
        loop {
            point_index = self.next_point_index_of(point_index);
            if self.point_is_endpoint(point_index) {
                return point_index;
            }
        }
    }

    #[inline]
    pub fn prev_point_index_of(&self, point_index: u32) -> u32 {
        if point_index == 0 {
            self.len() - 1
        } else {
            point_index - 1
        }
    }

    #[inline]
    pub fn next_point_index_of(&self, point_index: u32) -> u32 {
        if point_index == self.len() - 1 {
            0
        } else {
            point_index + 1
        }
    }

    pub fn transform(&mut self, transform: &Transform2F) {
        if transform.is_identity() {
            return;
        }

        for (point_index, point) in self.points.iter_mut().enumerate() {
            *point = *transform * *point;
            union_rect(&mut self.bounds, *point, point_index == 0);
        }
    }

    pub fn apply_perspective(&mut self, perspective: &Perspective) {
        for (point_index, point) in self.points.iter_mut().enumerate() {
            *point = *perspective * *point;
            union_rect(&mut self.bounds, *point, point_index == 0);
        }
    }

    pub fn dilate(&mut self, amount: Vector2F, orientation: Orientation) {
        ContourDilator::new(self, amount, orientation).dilate();
        self.bounds = self.bounds.dilate(amount);
    }

    fn prepare_for_tiling(&mut self, view_box: RectF) {
        // Snap points to the view box bounds. This mops up floating point error from the clipping
        // process.
        let (mut last_endpoint_index, mut contour_is_monotonic) = (None, true);
        for point_index in 0..(self.points.len() as u32) {
            if contour_is_monotonic {
                if self.point_is_endpoint(point_index) {
                    if let Some(last_endpoint_index) = last_endpoint_index {
                        if !self.curve_with_endpoints_is_monotonic(last_endpoint_index,
                                                                   point_index) {
                            contour_is_monotonic = false;
                        }
                    }
                    last_endpoint_index = Some(point_index);
                }
            }
        }

        // Convert to monotonic, if necessary.
        if !contour_is_monotonic {
            self.make_monotonic();
        }

        // Update bounds.
        self.bounds = self
            .bounds
            .intersection(view_box)
            .unwrap_or_else(|| RectF::default());
    }

    fn make_monotonic(&mut self) {
        debug!("--- make_monotonic() ---");

        let contour = self.take();
        self.bounds = contour.bounds;

        let mut last_endpoint_index = None;
        let input_point_count = contour.points.len() as u32;
        for point_index in 0..(input_point_count + 1) {
            if point_index < input_point_count && !contour.point_is_endpoint(point_index) {
                continue;
            }

            if let Some(last_endpoint_index) = last_endpoint_index {
                let position_index = if point_index == input_point_count {
                    0
                } else {
                    point_index
                };
                let baseline = LineSegment2F::new(
                    contour.points[last_endpoint_index as usize],
                    contour.points[position_index as usize],
                );
                let point_count = point_index - last_endpoint_index + 1;
                if point_count == 3 {
                    let ctrl_point_index = last_endpoint_index as usize + 1;
                    let ctrl_position = &contour.points[ctrl_point_index];
                    handle_cubic(
                        self,
                        &Segment::quadratic(baseline, *ctrl_position).to_cubic(),
                    );
                } else if point_count == 4 {
                    let first_ctrl_point_index = last_endpoint_index as usize + 1;
                    let ctrl_position_0 = &contour.points[first_ctrl_point_index + 0];
                    let ctrl_position_1 = &contour.points[first_ctrl_point_index + 1];
                    let ctrl = LineSegment2F::new(*ctrl_position_0, *ctrl_position_1);
                    handle_cubic(self, &Segment::cubic(baseline, ctrl));
                }

                self.push_point(
                    contour.points[position_index as usize],
                    PointFlags::empty(),
                    false,
                );
            }

            last_endpoint_index = Some(point_index);
        }

        fn handle_cubic(contour: &mut Contour, segment: &Segment) {
            debug!("handle_cubic({:?})", segment);

            match segment.as_cubic_segment().y_extrema() {
                (Some(t0), Some(t1)) => {
                    let (segments_01, segment_2) = segment.as_cubic_segment().split(t1);
                    let (segment_0, segment_1) = segments_01.as_cubic_segment().split(t0 / t1);
                    contour.push_segment(&segment_0, PushSegmentFlags::empty());
                    contour.push_segment(&segment_1, PushSegmentFlags::empty());
                    contour.push_segment(&segment_2, PushSegmentFlags::empty());
                }
                (Some(t0), None) | (None, Some(t0)) => {
                    let (segment_0, segment_1) = segment.as_cubic_segment().split(t0);
                    contour.push_segment(&segment_0, PushSegmentFlags::empty());
                    contour.push_segment(&segment_1, PushSegmentFlags::empty());
                }
                (None, None) => contour.push_segment(segment, PushSegmentFlags::empty()),
            }
        }
    }

    fn curve_with_endpoints_is_monotonic(
        &self,
        start_endpoint_index: u32,
        end_endpoint_index: u32,
    ) -> bool {
        let start_position = self.points[start_endpoint_index as usize];
        let end_position = self.points[end_endpoint_index as usize];

        if start_position.x() <= end_position.x() {
            for point_index in start_endpoint_index..end_endpoint_index {
                if self.points[point_index as usize].x() > self.points[point_index as usize + 1].x()
                {
                    return false;
                }
            }
        } else {
            for point_index in start_endpoint_index..end_endpoint_index {
                if self.points[point_index as usize].x() < self.points[point_index as usize + 1].x()
                {
                    return false;
                }
            }
        }

        if start_position.y() <= end_position.y() {
            for point_index in start_endpoint_index..end_endpoint_index {
                if self.points[point_index as usize].y() > self.points[point_index as usize + 1].y()
                {
                    return false;
                }
            }
        } else {
            for point_index in start_endpoint_index..end_endpoint_index {
                if self.points[point_index as usize].y() < self.points[point_index as usize + 1].y()
                {
                    return false;
                }
            }
        }

        true
    }

    // Use this function to keep bounds up to date when mutating paths. See `Outline::transform()`
    // for an example of use.
    pub(crate) fn update_bounds(&self, bounds: &mut Option<RectF>) {
        *bounds = Some(match *bounds {
            None => self.bounds,
            Some(bounds) => bounds.union_rect(self.bounds),
        })
    }
}

impl Debug for Contour {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        for (segment_index, segment) in self.iter(ContourIterFlags::IGNORE_CLOSE_SEGMENT)
                                            .enumerate() {
            if segment_index == 0 {
                write!(
                    formatter,
                    "M {} {}",
                    segment.baseline.from_x(),
                    segment.baseline.from_y()
                )?;
            }

            match segment.kind {
                SegmentKind::None => {}
                SegmentKind::Line => {
                    write!(
                        formatter,
                        " L {} {}",
                        segment.baseline.to_x(),
                        segment.baseline.to_y()
                    )?;
                }
                SegmentKind::Quadratic => {
                    write!(
                        formatter,
                        " Q {} {} {} {}",
                        segment.ctrl.from_x(),
                        segment.ctrl.from_y(),
                        segment.baseline.to_x(),
                        segment.baseline.to_y()
                    )?;
                }
                SegmentKind::Cubic => {
                    write!(
                        formatter,
                        " C {} {} {} {} {} {}",
                        segment.ctrl.from_x(),
                        segment.ctrl.from_y(),
                        segment.ctrl.to_x(),
                        segment.ctrl.to_y(),
                        segment.baseline.to_x(),
                        segment.baseline.to_y()
                    )?;
                }
            }
        }

        if self.closed {
            write!(formatter, " z")?;
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct PointIndex(u32);

impl PointIndex {
    #[inline]
    pub fn new(contour: u32, point: u32) -> PointIndex {
        debug_assert!(contour <= 0xfff);
        debug_assert!(point <= 0x000f_ffff);
        PointIndex((contour << 20) | point)
    }

    #[inline]
    pub fn contour(self) -> u32 {
        self.0 >> 20
    }

    #[inline]
    pub fn point(self) -> u32 {
        self.0 & 0x000f_ffff
    }
}

pub struct ContourIter<'a> {
    contour: &'a Contour,
    index: u32,
    flags: ContourIterFlags,
}

impl<'a> Iterator for ContourIter<'a> {
    type Item = Segment;

    #[inline]
    fn next(&mut self) -> Option<Segment> {
        let contour = self.contour;

        let include_close_segment = self.contour.closed &&
            !self.flags.contains(ContourIterFlags::IGNORE_CLOSE_SEGMENT);
        if (self.index == contour.len() && !include_close_segment) ||
                self.index == contour.len() + 1 {
            return None;
        }

        let point0_index = self.index - 1;
        let point0 = contour.position_of(point0_index);
        if self.index == contour.len() {
            let point1 = contour.position_of(0);
            self.index += 1;
            return Some(Segment::line(LineSegment2F::new(point0, point1)));
        }

        let point1_index = self.index;
        self.index += 1;
        let point1 = contour.position_of(point1_index);
        if contour.point_is_endpoint(point1_index) {
            return Some(Segment::line(LineSegment2F::new(point0, point1)));
        }

        let point2_index = self.index;
        let point2 = contour.position_of(point2_index);
        self.index += 1;
        if contour.point_is_endpoint(point2_index) {
            return Some(Segment::quadratic(LineSegment2F::new(point0, point2), point1));
        }

        let point3_index = self.index;
        let point3 = contour.position_of(point3_index);
        self.index += 1;
        debug_assert!(contour.point_is_endpoint(point3_index));
        return Some(Segment::cubic(
            LineSegment2F::new(point0, point3),
            LineSegment2F::new(point1, point2),
        ));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArcDirection {
    CW,
    CCW,
}

bitflags! {
    pub struct ContourIterFlags: u8 {
        const IGNORE_CLOSE_SEGMENT = 1;
    }
}

#[inline]
pub(crate) fn union_rect(bounds: &mut RectF, new_point: Vector2F, first: bool) {
    if first {
        *bounds = RectF::from_points(new_point, new_point);
    } else {
        *bounds = bounds.union_point(new_point)
    }
}
