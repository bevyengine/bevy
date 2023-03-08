use crate::{IVec2, UVec2, Vec2};

macro_rules! create_rect_type {
    ($type_name:ident, $vec_type:ty, $num_type:ty) => {
        /// A rectangle defined by two opposite corners.
        ///
        /// The rectangle is axis aligned, and defined by its minimum and maximum coordinates,
        #[doc = concat!(" stored in `", stringify!($type_name), "::min` and `", stringify!($type_name), "::max`, respectively. The minimum/maximum invariant")]
        /// must be upheld by the user when directly assigning the fields, otherwise some methods
        /// produce invalid results. It is generally recommended to use one of the constructor
        /// methods instead, which will ensure this invariant is met, unless you already have
        /// the minimum and maximum corners.
        #[repr(C)]
        #[derive(Default, Clone, Copy, Debug, PartialEq)]
        #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
        pub struct $type_name {
            /// The minimum corner point of the rect.
            pub min: $vec_type,
            /// The maximum corner point of the rect.
            pub max: $vec_type,
        }

        impl $type_name {
            /// Create a new rectangle from two corner points.
            ///
            /// The two points do not need to be the minimum and/or maximum corners.
            /// They only need to be two opposite corners.
            #[inline]
            pub fn new(x0: $num_type, y0: $num_type, x1: $num_type, y1: $num_type) -> Self {
                Self::from_corners(<$vec_type>::new(x0, y0), <$vec_type>::new(x1, y1))
            }

            /// Create a new rectangle from two corner points.
            ///
            /// The two points do not need to be the minimum and/or maximum corners.
            /// They only need to be two opposite corners.
            #[inline]
            pub fn from_corners(p0: $vec_type, p1: $vec_type) -> Self {
                Self {
                    min: p0.min(p1),
                    max: p0.max(p1),
                }
            }

            /// Create a new rectangle from its center and size.
            ///
            /// # Panics
            ///
            /// This method panics if any of the components of the size is negative.
            #[inline]
            pub fn from_center_size(origin: $vec_type, size: $vec_type) -> Self {
                assert!(size.cmpge(<$vec_type>::ZERO).all());
                let half_size = size / 2 as $num_type;
                Self::from_center_half_size(origin, half_size)
            }

            /// Create a new rectangle from its center and half-size.
            ///
            /// # Panics
            ///
            /// This method panics if any of the components of the half-size is negative.
            #[inline]
            pub fn from_center_half_size(origin: $vec_type, half_size: $vec_type) -> Self {
                assert!(half_size.cmpge(<$vec_type>::ZERO).all());
                Self {
                    min: origin - half_size,
                    max: origin + half_size,
                }
            }

            /// Check if the rectangle is empty.
            #[inline]
            pub fn is_empty(&self) -> bool {
                self.min.cmpge(self.max).any()
            }

            /// Rectangle width (max.x - min.x).
            #[inline]
            pub fn width(&self) -> $num_type {
                self.max.x - self.min.x
            }

            /// Rectangle height (max.y - min.y).
            #[inline]
            pub fn height(&self) -> $num_type {
                self.max.y - self.min.y
            }

            /// Rectangle size.
            #[inline]
            pub fn size(&self) -> $vec_type {
                self.max - self.min
            }

            /// Rectangle half-size.
            #[inline]
            pub fn half_size(&self) -> $vec_type {
                self.size() / 2 as $num_type
            }

            /// The center point of the rectangle.
            #[inline]
            pub fn center(&self) -> $vec_type {
                (self.min + self.max) / 2 as $num_type
            }

            /// Check if a point lies within this rectangle, inclusive of its edges.
            #[inline]
            pub fn contains(&self, point: $vec_type) -> bool {
                (point.cmpge(self.min) & point.cmple(self.max)).all()
            }

            /// Build a new rectangle formed of the union of this rectangle and another rectangle.
            ///
            /// The union is the smallest rectangle enclosing both rectangles.
            #[inline]
            pub fn union(&self, other: Self) -> Self {
                Self {
                    min: self.min.min(other.min),
                    max: self.max.max(other.max),
                }
            }

            /// Build a new rectangle formed of the union of this rectangle and a point.
            ///
            /// The union is the smallest rectangle enclosing both the rectangle and the point. If the
            /// point is already inside the rectangle, this method returns a copy of the rectangle.
            #[inline]
            pub fn union_point(&self, other: $vec_type) -> Self {
                Self {
                    min: self.min.min(other),
                    max: self.max.max(other),
                }
            }

            /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
            ///
            /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
            #[doc = concat!(" is empty, this method returns an empty rectangle ([`", stringify!($type_name), "::is_empty()`] returns `true`), but")]
            #[doc = concat!(" the actual values of [`", stringify!($type_name), "::min`] and [`", stringify!($type_name), "::max`] are implementation-dependent.")]
            #[inline]
            pub fn intersect(&self, other: Self) -> Self {
                let mut r = Self {
                    min: self.min.max(other.min),
                    max: self.max.min(other.max),
                };
                // Collapse min over max to enforce invariants and ensure e.g. width() or
                // height() never return a negative value.
                r.min = r.min.min(r.max);
                r
            }

            /// Create a new rectangle with a constant inset.
            ///
            /// The inset is the extra border on all sides. A positive inset produces a larger rectangle,
            /// while a negative inset is allowed and produces a smaller rectangle. If the inset is negative
            /// and its absolute value is larger than the rectangle half-size, the created rectangle is empty.
            #[inline]
            pub fn inset(&self, inset: $num_type) -> Self {
                let mut r = Self {
                    min: self.min - inset,
                    max: self.max + inset,
                };
                // Collapse min over max to enforce invariants and ensure e.g. width() or
                // height() never return a negative value.
                r.min = r.min.min(r.max);
                r
            }
        }
    };
}

create_rect_type!(Rect, Vec2, f32);
create_rect_type!(IRect, IVec2, i32);
create_rect_type!(URect, UVec2, u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed() {
        let r = Rect::from_center_size(Vec2::new(3., -5.), Vec2::new(8., 11.));

        assert!(r.min.abs_diff_eq(Vec2::new(-1., -10.5), 1e-5));
        assert!(r.max.abs_diff_eq(Vec2::new(7., 0.5), 1e-5));

        assert!(r.center().abs_diff_eq(Vec2::new(3., -5.), 1e-5));

        assert!((r.width() - 8.).abs() <= 1e-5);
        assert!((r.height() - 11.).abs() <= 1e-5);
        assert!(r.size().abs_diff_eq(Vec2::new(8., 11.), 1e-5));
        assert!(r.half_size().abs_diff_eq(Vec2::new(4., 5.5), 1e-5));

        assert!(r.contains(Vec2::new(3., -5.)));
        assert!(r.contains(Vec2::new(-1., -10.5)));
        assert!(r.contains(Vec2::new(-1., 0.5)));
        assert!(r.contains(Vec2::new(7., -10.5)));
        assert!(r.contains(Vec2::new(7., 0.5)));
        assert!(!r.contains(Vec2::new(50., -5.)));
    }

    #[test]
    fn rect_union() {
        let r = Rect::from_center_size(Vec2::ZERO, Vec2::ONE); // [-0.5,-0.5] - [0.5,0.5]

        // overlapping
        let r2 = Rect {
            min: Vec2::new(-0.8, 0.3),
            max: Vec2::new(0.1, 0.7),
        };
        let u = r.union(r2);
        assert!(u.min.abs_diff_eq(Vec2::new(-0.8, -0.5), 1e-5));
        assert!(u.max.abs_diff_eq(Vec2::new(0.5, 0.7), 1e-5));

        // disjoint
        let r2 = Rect {
            min: Vec2::new(-1.8, -0.5),
            max: Vec2::new(-1.5, 0.3),
        };
        let u = r.union(r2);
        assert!(u.min.abs_diff_eq(Vec2::new(-1.8, -0.5), 1e-5));
        assert!(u.max.abs_diff_eq(Vec2::new(0.5, 0.5), 1e-5));

        // included
        let r2 = Rect::from_center_size(Vec2::ZERO, Vec2::splat(0.5));
        let u = r.union(r2);
        assert!(u.min.abs_diff_eq(r.min, 1e-5));
        assert!(u.max.abs_diff_eq(r.max, 1e-5));

        // including
        let r2 = Rect::from_center_size(Vec2::ZERO, Vec2::splat(1.5));
        let u = r.union(r2);
        assert!(u.min.abs_diff_eq(r2.min, 1e-5));
        assert!(u.max.abs_diff_eq(r2.max, 1e-5));
    }

    #[test]
    fn rect_union_pt() {
        let r = Rect::from_center_size(Vec2::ZERO, Vec2::ONE); // [-0.5,-0.5] - [0.5,0.5]

        // inside
        let v = Vec2::new(0.3, -0.2);
        let u = r.union_point(v);
        assert!(u.min.abs_diff_eq(r.min, 1e-5));
        assert!(u.max.abs_diff_eq(r.max, 1e-5));

        // outside
        let v = Vec2::new(10., -3.);
        let u = r.union_point(v);
        assert!(u.min.abs_diff_eq(Vec2::new(-0.5, -3.), 1e-5));
        assert!(u.max.abs_diff_eq(Vec2::new(10., 0.5), 1e-5));
    }

    #[test]
    fn rect_intersect() {
        let r = Rect::from_center_size(Vec2::ZERO, Vec2::ONE); // [-0.5,-0.5] - [0.5,0.5]

        // overlapping
        let r2 = Rect {
            min: Vec2::new(-0.8, 0.3),
            max: Vec2::new(0.1, 0.7),
        };
        let u = r.intersect(r2);
        assert!(u.min.abs_diff_eq(Vec2::new(-0.5, 0.3), 1e-5));
        assert!(u.max.abs_diff_eq(Vec2::new(0.1, 0.5), 1e-5));

        // disjoint
        let r2 = Rect {
            min: Vec2::new(-1.8, -0.5),
            max: Vec2::new(-1.5, 0.3),
        };
        let u = r.intersect(r2);
        assert!(u.is_empty());
        assert!(u.width() <= 1e-5);

        // included
        let r2 = Rect::from_center_size(Vec2::ZERO, Vec2::splat(0.5));
        let u = r.intersect(r2);
        assert!(u.min.abs_diff_eq(r2.min, 1e-5));
        assert!(u.max.abs_diff_eq(r2.max, 1e-5));

        // including
        let r2 = Rect::from_center_size(Vec2::ZERO, Vec2::splat(1.5));
        let u = r.intersect(r2);
        assert!(u.min.abs_diff_eq(r.min, 1e-5));
        assert!(u.max.abs_diff_eq(r.max, 1e-5));
    }

    #[test]
    fn rect_inset() {
        let r = Rect::from_center_size(Vec2::ZERO, Vec2::ONE); // [-0.5,-0.5] - [0.5,0.5]

        let r2 = r.inset(0.3);
        assert!(r2.min.abs_diff_eq(Vec2::new(-0.8, -0.8), 1e-5));
        assert!(r2.max.abs_diff_eq(Vec2::new(0.8, 0.8), 1e-5));
    }
}
