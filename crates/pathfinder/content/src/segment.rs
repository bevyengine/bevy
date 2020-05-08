// pathfinder/content/src/segment.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Line or curve segments, optimized with SIMD.

use pathfinder_geometry::line_segment::LineSegment2F;
use pathfinder_geometry::transform2d::Transform2F;
use pathfinder_geometry::util::{self, EPSILON};
use pathfinder_geometry::vector::{Vector2F, vec2f};
use pathfinder_simd::default::F32x4;
use std::f32::consts::SQRT_2;

const MAX_NEWTON_ITERATIONS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub baseline: LineSegment2F,
    pub ctrl: LineSegment2F,
    pub kind: SegmentKind,
    pub flags: SegmentFlags,
}

impl Segment {
    #[inline]
    pub fn none() -> Segment {
        Segment {
            baseline: LineSegment2F::default(),
            ctrl: LineSegment2F::default(),
            kind: SegmentKind::None,
            flags: SegmentFlags::empty(),
        }
    }

    #[inline]
    pub fn line(line: LineSegment2F) -> Segment {
        Segment {
            baseline: line,
            ctrl: LineSegment2F::default(),
            kind: SegmentKind::Line,
            flags: SegmentFlags::empty(),
        }
    }

    #[inline]
    pub fn quadratic(baseline: LineSegment2F, ctrl: Vector2F) -> Segment {
        Segment {
            baseline,
            ctrl: LineSegment2F::new(ctrl, Vector2F::zero()),
            kind: SegmentKind::Quadratic,
            flags: SegmentFlags::empty(),
        }
    }

    #[inline]
    pub fn cubic(baseline: LineSegment2F, ctrl: LineSegment2F) -> Segment {
        Segment {
            baseline,
            ctrl,
            kind: SegmentKind::Cubic,
            flags: SegmentFlags::empty(),
        }
    }

    /// Approximates an unit-length arc with a cubic Bézier curve.
    ///
    /// The maximum supported sweep angle is π/2 (i.e. 90°).
    pub fn arc(sweep_angle: f32) -> Segment {
        Segment::arc_from_cos(f32::cos(sweep_angle))
    }

    /// Approximates an unit-length arc with a cubic Bézier curve, given the cosine of the sweep
    /// angle.
    ///
    /// The maximum supported sweep angle is π/2 (i.e. 90°).
    pub fn arc_from_cos(cos_sweep_angle: f32) -> Segment {
        // Richard A. DeVeneza, "How to determine the control points of a Bézier curve that
        // approximates a small arc", 2004.
        //
        // https://www.tinaja.com/glib/bezcirc2.pdf
        if cos_sweep_angle >= 1.0 - EPSILON {
            return Segment::line(LineSegment2F::new(vec2f(1.0, 0.0), vec2f(1.0, 0.0)));
        }

        let term = F32x4::new(cos_sweep_angle, -cos_sweep_angle,
                              cos_sweep_angle, -cos_sweep_angle);
        let signs = F32x4::new(1.0, -1.0, 1.0, 1.0);
        let p3p0 = ((F32x4::splat(1.0) + term) * F32x4::splat(0.5)).sqrt() * signs;
        let (p0x, p0y) = (p3p0.z(), p3p0.w());
        let (p1x, p1y) = (4.0 - p0x, (1.0 - p0x) * (3.0 - p0x) / p0y);
        let p2p1 = F32x4::new(p1x, -p1y, p1x, p1y) * F32x4::splat(1.0 / 3.0);
        return Segment::cubic(LineSegment2F(p3p0), LineSegment2F(p2p1));
    }

    #[inline]
    pub fn quarter_circle_arc() -> Segment {
        let p0 = Vector2F::splat(SQRT_2 * 0.5);
        let p1 = vec2f(-SQRT_2 / 6.0 + 4.0 / 3.0, 7.0 * SQRT_2 / 6.0 - 4.0 / 3.0);
        let flip = vec2f(1.0, -1.0);
        let (p2, p3) = (p1 * flip, p0 * flip);
        Segment::cubic(LineSegment2F::new(p3, p0), LineSegment2F::new(p2, p1))
    }

    #[inline]
    pub fn as_line_segment(&self) -> LineSegment2F {
        debug_assert!(self.is_line());
        self.baseline
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self.kind == SegmentKind::None
    }

    #[inline]
    pub fn is_line(&self) -> bool {
        self.kind == SegmentKind::Line
    }

    #[inline]
    pub fn is_quadratic(&self) -> bool {
        self.kind == SegmentKind::Quadratic
    }

    #[inline]
    pub fn is_cubic(&self) -> bool {
        self.kind == SegmentKind::Cubic
    }

    #[inline]
    pub fn as_cubic_segment(&self) -> CubicSegment {
        debug_assert!(self.is_cubic());
        CubicSegment(self)
    }

    // FIXME(pcwalton): We should basically never use this function.
    // FIXME(pcwalton): Handle lines!
    #[inline]
    pub fn to_cubic(&self) -> Segment {
        if self.is_cubic() {
            return *self;
        }

        let mut new_segment = *self;
        let p1_2 = self.ctrl.from() + self.ctrl.from();
        new_segment.ctrl = LineSegment2F::new(self.baseline.from() + p1_2,
                                              p1_2 + self.baseline.to()) * (1.0 / 3.0);
        new_segment.kind = SegmentKind::Cubic;
        new_segment
    }

    #[inline]
    pub fn is_monotonic(&self) -> bool {
        // FIXME(pcwalton): Don't degree elevate!
        match self.kind {
            SegmentKind::None | SegmentKind::Line => true,
            SegmentKind::Quadratic => self.to_cubic().as_cubic_segment().is_monotonic(),
            SegmentKind::Cubic => self.as_cubic_segment().is_monotonic(),
        }
    }

    #[inline]
    pub fn reversed(&self) -> Segment {
        Segment {
            baseline: self.baseline.reversed(),
            ctrl: if self.is_quadratic() {
                self.ctrl
            } else {
                self.ctrl.reversed()
            },
            kind: self.kind,
            flags: self.flags,
        }
    }

    // Reverses if necessary so that the from point is above the to point. Calling this method
    // again will undo the transformation.
    #[inline]
    pub fn orient(&self, y_winding: i32) -> Segment {
        if y_winding >= 0 {
            *self
        } else {
            self.reversed()
        }
    }

    #[inline]
    pub fn is_tiny(&self) -> bool {
        const EPSILON: f32 = 0.0001;
        self.baseline.square_length() < EPSILON
    }

    #[inline]
    pub fn split(&self, t: f32) -> (Segment, Segment) {
        // FIXME(pcwalton): Don't degree elevate!
        if self.is_line() {
            let (before, after) = self.as_line_segment().split(t);
            (Segment::line(before), Segment::line(after))
        } else {
            self.to_cubic().as_cubic_segment().split(t)
        }
    }

    #[inline]
    pub fn sample(self, t: f32) -> Vector2F {
        // FIXME(pcwalton): Don't degree elevate!
        if self.is_line() {
            self.as_line_segment().sample(t)
        } else {
            self.to_cubic().as_cubic_segment().sample(t)
        }
    }

    #[inline]
    pub fn transform(self, transform: &Transform2F) -> Segment {
        Segment {
            baseline: *transform * self.baseline,
            ctrl: *transform * self.ctrl,
            kind: self.kind,
            flags: self.flags,
        }
    }

    pub fn arc_length(&self) -> f32 {
        // FIXME(pcwalton)
        self.baseline.vector().length()
    }

    pub fn time_for_distance(&self, distance: f32) -> f32 {
        // FIXME(pcwalton)
        distance / self.arc_length()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum SegmentKind {
    None,
    Line,
    Quadratic,
    Cubic,
}

bitflags! {
    pub struct SegmentFlags: u8 {
        const FIRST_IN_SUBPATH = 0x01;
        const CLOSES_SUBPATH = 0x02;
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CubicSegment<'s>(pub &'s Segment);

impl<'s> CubicSegment<'s> {
    // See Kaspar Fischer, "Piecewise Linear Approximation of Bézier Curves", 2000.
    #[inline]
    pub fn is_flat(self, tolerance: f32) -> bool {
        let mut uv = F32x4::splat(3.0) * self.0.ctrl.0
            - self.0.baseline.0
            - self.0.baseline.0
            - self.0.baseline.reversed().0;
        uv = uv * uv;
        uv = uv.max(uv.zwxy());
        uv[0] + uv[1] <= 16.0 * tolerance * tolerance
    }

    #[inline]
    pub fn split(self, t: f32) -> (Segment, Segment) {
        let (baseline0, ctrl0, baseline1, ctrl1);
        if t <= 0.0 {
            let from = &self.0.baseline.from();
            baseline0 = LineSegment2F::new(*from, *from);
            ctrl0 = LineSegment2F::new(*from, *from);
            baseline1 = self.0.baseline;
            ctrl1 = self.0.ctrl;
        } else if t >= 1.0 {
            let to = &self.0.baseline.to();
            baseline0 = self.0.baseline;
            ctrl0 = self.0.ctrl;
            baseline1 = LineSegment2F::new(*to, *to);
            ctrl1 = LineSegment2F::new(*to, *to);
        } else {
            let tttt = F32x4::splat(t);

            let (p0p3, p1p2) = (self.0.baseline.0, self.0.ctrl.0);
            let p0p1 = p0p3.concat_xy_xy(p1p2);

            // p01 = lerp(p0, p1, t), p12 = lerp(p1, p2, t), p23 = lerp(p2, p3, t)
            let p01p12 = p0p1 + tttt * (p1p2 - p0p1);
            let pxxp23 = p1p2 + tttt * (p0p3 - p1p2);
            let p12p23 = p01p12.concat_zw_zw(pxxp23);

            // p012 = lerp(p01, p12, t), p123 = lerp(p12, p23, t)
            let p012p123 = p01p12 + tttt * (p12p23 - p01p12);
            let p123 = p012p123.zwzw();

            // p0123 = lerp(p012, p123, t)
            let p0123 = p012p123 + tttt * (p123 - p012p123);

            baseline0 = LineSegment2F(p0p3.concat_xy_xy(p0123));
            ctrl0 = LineSegment2F(p01p12.concat_xy_xy(p012p123));
            baseline1 = LineSegment2F(p0123.concat_xy_zw(p0p3));
            ctrl1 = LineSegment2F(p012p123.concat_zw_zw(p12p23));
        }

        (
            Segment {
                baseline: baseline0,
                ctrl: ctrl0,
                kind: SegmentKind::Cubic,
                flags: self.0.flags & SegmentFlags::FIRST_IN_SUBPATH,
            },
            Segment {
                baseline: baseline1,
                ctrl: ctrl1,
                kind: SegmentKind::Cubic,
                flags: self.0.flags & SegmentFlags::CLOSES_SUBPATH,
            },
        )
    }

    #[inline]
    pub fn split_before(self, t: f32) -> Segment {
        self.split(t).0
    }

    #[inline]
    pub fn split_after(self, t: f32) -> Segment {
        self.split(t).1
    }

    // FIXME(pcwalton): Use Horner's method!
    #[inline]
    pub fn sample(self, t: f32) -> Vector2F {
        self.split(t).0.baseline.to()
    }

    #[inline]
    pub fn is_monotonic(self) -> bool {
        // TODO(pcwalton): Optimize this.
        let (p0, p3) = (self.0.baseline.from_y(), self.0.baseline.to_y());
        let (p1, p2) = (self.0.ctrl.from_y(), self.0.ctrl.to_y());
        (p0 <= p1 && p1 <= p2 && p2 <= p3) || (p0 >= p1 && p1 >= p2 && p2 >= p3)
    }

    #[inline]
    pub fn y_extrema(self) -> (Option<f32>, Option<f32>) {
        if self.is_monotonic() {
            return (None, None);
        }

        let p0p1p2p3 = F32x4::new(
            self.0.baseline.from_y(),
            self.0.ctrl.from_y(),
            self.0.ctrl.to_y(),
            self.0.baseline.to_y(),
        );

        let pxp0p1p2 = p0p1p2p3.wxyz();
        let pxv0v1v2 = p0p1p2p3 - pxp0p1p2;
        let (v0, v1, v2) = (pxv0v1v2[1], pxv0v1v2[2], pxv0v1v2[3]);

        let (t0, t1);
        let (v0_to_v1, v2_to_v1) = (v0 - v1, v2 - v1);
        let denom = v0_to_v1 + v2_to_v1;

        if util::approx_eq(denom, 0.0) {
            // Let's not divide by zero (issue #146). Fall back to Newton's method.
            // FIXME(pcwalton): Can we have two roots here?
            let mut t = 0.5;
            for _ in 0..MAX_NEWTON_ITERATIONS {
                let dydt = 3.0 * ((denom * t - v0_to_v1 - v0_to_v1) * t + v0);
                if f32::abs(dydt) <= EPSILON {
                    break
                }
                let d2ydt2 = 6.0 * (denom * t - v0_to_v1);
                t -= dydt / d2ydt2;
            }
            t0 = t;
            t1 = 0.0;
            debug!("...  t=(newton) {}", t);
        } else {
            // Algebraically compute the values for t.
            let discrim = f32::sqrt(v1 * v1 - v0 * v2);
            let denom_recip = 1.0 / denom;

            t0 = (v0_to_v1 + discrim) * denom_recip;
            t1 = (v0_to_v1 - discrim) * denom_recip;

            debug!("... t=({} +/- {})/{} t0={} t1={}", v0_to_v1, discrim, denom, t0, t1);
        }

        return match (
            t0 > EPSILON && t0 < 1.0 - EPSILON,
            t1 > EPSILON && t1 < 1.0 - EPSILON,
        ) {
            (false, false) => (None, None),
            (true, false) => (Some(t0), None),
            (false, true) => (Some(t1), None),
            (true, true) => (Some(f32::min(t0, t1)), Some(f32::max(t0, t1))),
        };
    }

    #[inline]
    pub fn min_x(&self) -> f32 {
        f32::min(self.0.baseline.min_x(), self.0.ctrl.min_x())
    }
    #[inline]
    pub fn min_y(&self) -> f32 {
        f32::min(self.0.baseline.min_y(), self.0.ctrl.min_y())
    }
    #[inline]
    pub fn max_x(&self) -> f32 {
        f32::max(self.0.baseline.max_x(), self.0.ctrl.max_x())
    }
    #[inline]
    pub fn max_y(&self) -> f32 {
        f32::max(self.0.baseline.max_y(), self.0.ctrl.max_y())
    }
}
