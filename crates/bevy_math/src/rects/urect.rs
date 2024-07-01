use crate::{IRect, Rect, UVec2};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A rectangle defined by two opposite corners.
///
/// The rectangle is axis aligned, and defined by its minimum and maximum coordinates,
/// stored in `URect::min` and `URect::max`, respectively. The minimum/maximum invariant
/// must be upheld by the user when directly assigning the fields, otherwise some methods
/// produce invalid results. It is generally recommended to use one of the constructor
/// methods instead, which will ensure this invariant is met, unless you already have
/// the minimum and maximum corners.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Default)
)]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct URect {
    /// The minimum corner point of the rect.
    pub min: UVec2,
    /// The maximum corner point of the rect.
    pub max: UVec2,
}

impl URect {
    /// An empty `URect`, represented by maximum and minimum corner points
    /// with `max == UVec2::MIN` and `min == UVec2::MAX`, so the
    /// rect has an extremely large negative size.
    /// This is useful, because when taking a union B of a non-empty `URect` A and
    /// this empty `URect`, B will simply equal A.
    pub const EMPTY: Self = Self {
        max: UVec2::MIN,
        min: UVec2::MAX,
    };
    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 4, 10, 6); // w=10 h=2
    /// let r = URect::new(2, 4, 5, 0); // w=3 h=4
    /// ```
    #[inline]
    pub fn new(x0: u32, y0: u32, x1: u32, y1: u32) -> Self {
        Self::from_corners(UVec2::new(x0, y0), UVec2::new(x1, y1))
    }

    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// // Unit rect from [0,0] to [1,1]
    /// let r = URect::from_corners(UVec2::ZERO, UVec2::ONE); // w=1 h=1
    /// // Same; the points do not need to be ordered
    /// let r = URect::from_corners(UVec2::ONE, UVec2::ZERO); // w=1 h=1
    /// ```
    #[inline]
    pub fn from_corners(p0: UVec2, p1: UVec2) -> Self {
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    /// Create a new rectangle from its center and size.
    ///
    /// # Rounding Behaviour
    ///
    /// If the size contains odd numbers they will be rounded down to the nearest whole number.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the size is negative or if `origin - (size / 2)` results in any negatives.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::from_center_size(UVec2::ONE, UVec2::splat(2)); // w=2 h=2
    /// assert_eq!(r.min, UVec2::splat(0));
    /// assert_eq!(r.max, UVec2::splat(2));
    /// ```
    #[inline]
    pub fn from_center_size(origin: UVec2, size: UVec2) -> Self {
        assert!(origin.cmpge(size / 2).all(), "Origin must always be greater than or equal to (size / 2) otherwise the rectangle is undefined! Origin was {origin} and size was {size}");
        let half_size = size / 2;
        Self::from_center_half_size(origin, half_size)
    }

    /// Create a new rectangle from its center and half-size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the half-size is negative or if `origin - half_size` results in any negatives.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::from_center_half_size(UVec2::ONE, UVec2::ONE); // w=2 h=2
    /// assert_eq!(r.min, UVec2::splat(0));
    /// assert_eq!(r.max, UVec2::splat(2));
    /// ```
    #[inline]
    pub fn from_center_half_size(origin: UVec2, half_size: UVec2) -> Self {
        assert!(origin.cmpge(half_size).all(), "Origin must always be greater than or equal to half_size otherwise the rectangle is undefined! Origin was {origin} and half_size was {half_size}");
        Self {
            min: origin - half_size,
            max: origin + half_size,
        }
    }

    /// Check if the rectangle is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::from_corners(UVec2::ZERO, UVec2::new(0, 1)); // w=0 h=1
    /// assert!(r.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.cmpge(self.max).any()
    }

    /// Rectangle width (max.x - min.x).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.width(), 5);
    /// ```
    #[inline]
    pub const fn width(&self) -> u32 {
        self.max.x - self.min.x
    }

    /// Rectangle height (max.y - min.y).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.height(), 1);
    /// ```
    #[inline]
    pub const fn height(&self) -> u32 {
        self.max.y - self.min.y
    }

    /// Rectangle size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.size(), UVec2::new(5, 1));
    /// ```
    #[inline]
    pub fn size(&self) -> UVec2 {
        self.max - self.min
    }

    /// Rectangle half-size.
    ///
    /// # Rounding Behaviour
    ///
    /// If the full size contains odd numbers they will be rounded down to the nearest whole number when calculating the half size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(0, 0, 4, 2); // w=4 h=2
    /// assert_eq!(r.half_size(), UVec2::new(2, 1));
    /// ```
    #[inline]
    pub fn half_size(&self) -> UVec2 {
        self.size() / 2
    }

    /// The center point of the rectangle.
    ///
    /// # Rounding Behaviour
    ///
    /// If the (min + max) contains odd numbers they will be rounded down to the nearest whole number when calculating the center.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(0, 0, 4, 2); // w=4 h=2
    /// assert_eq!(r.center(), UVec2::new(2, 1));
    /// ```
    #[inline]
    pub fn center(&self) -> UVec2 {
        (self.min + self.max) / 2
    }

    /// Check if a point lies within this rectangle, inclusive of its edges.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::URect;
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// assert!(r.contains(r.center()));
    /// assert!(r.contains(r.min));
    /// assert!(r.contains(r.max));
    /// ```
    #[inline]
    pub fn contains(&self, point: UVec2) -> bool {
        (point.cmpge(self.min) & point.cmple(self.max)).all()
    }

    /// Build a new rectangle formed of the union of this rectangle and another rectangle.
    ///
    /// The union is the smallest rectangle enclosing both rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r1 = URect::new(0, 0, 5, 1); // w=5 h=1
    /// let r2 = URect::new(1, 0, 3, 8); // w=2 h=4
    /// let r = r1.union(r2);
    /// assert_eq!(r.min, UVec2::new(0, 0));
    /// assert_eq!(r.max, UVec2::new(5, 8));
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(0, 0, 5, 1); // w=5 h=1
    /// let u = r.union_point(UVec2::new(3, 6));
    /// assert_eq!(u.min, UVec2::ZERO);
    /// assert_eq!(u.max, UVec2::new(5, 6));
    /// ```
    #[inline]
    pub fn union_point(&self, other: UVec2) -> Self {
        Self {
            min: self.min.min(other),
            max: self.max.max(other),
        }
    }

    /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
    ///
    /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
    /// is empty, this method returns an empty rectangle ([`URect::is_empty()`] returns `true`), but
    /// the actual values of [`URect::min`] and [`URect::max`] are implementation-dependent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r1 = URect::new(0, 0, 2, 2); // w=2 h=2
    /// let r2 = URect::new(1, 1, 3, 3); // w=2 h=2
    /// let r = r1.intersect(r2);
    /// assert_eq!(r.min, UVec2::new(1, 1));
    /// assert_eq!(r.max, UVec2::new(2, 2));
    /// ```
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

    /// Create a new rectangle by expanding it evenly on all sides.
    ///
    /// A positive expansion value produces a larger rectangle,
    /// while a negative expansion value produces a smaller rectangle.
    /// If this would result in zero width or height, [`URect::EMPTY`] is returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{URect, UVec2};
    /// let r = URect::new(4, 4, 6, 6); // w=2 h=2
    /// let r2 = r.inflate(1); // w=4 h=4
    /// assert_eq!(r2.min, UVec2::splat(3));
    /// assert_eq!(r2.max, UVec2::splat(7));
    ///
    /// let r = URect::new(4, 4, 8, 8); // w=4 h=4
    /// let r2 = r.inflate(-1); // w=2 h=2
    /// assert_eq!(r2.min, UVec2::splat(5));
    /// assert_eq!(r2.max, UVec2::splat(7));
    /// ```
    #[inline]
    pub fn inflate(&self, expansion: i32) -> Self {
        let mut r = Self {
            min: UVec2::new(
                self.min.x.saturating_add_signed(-expansion),
                self.min.y.saturating_add_signed(-expansion),
            ),
            max: UVec2::new(
                self.max.x.saturating_add_signed(expansion),
                self.max.y.saturating_add_signed(expansion),
            ),
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }

    /// Returns self as [`Rect`] (f32)
    #[inline]
    pub fn as_rect(&self) -> Rect {
        Rect::from_corners(self.min.as_vec2(), self.max.as_vec2())
    }

    /// Returns self as [`IRect`] (i32)
    #[inline]
    pub fn as_irect(&self) -> IRect {
        IRect::from_corners(self.min.as_ivec2(), self.max.as_ivec2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed() {
        let r = URect::from_center_size(UVec2::new(10, 16), UVec2::new(8, 12));

        assert_eq!(r.min, UVec2::new(6, 10));
        assert_eq!(r.max, UVec2::new(14, 22));

        assert_eq!(r.center(), UVec2::new(10, 16));

        assert_eq!(r.width(), 8);
        assert_eq!(r.height(), 12);
        assert_eq!(r.size(), UVec2::new(8, 12));
        assert_eq!(r.half_size(), UVec2::new(4, 6));

        assert!(r.contains(UVec2::new(7, 10)));
        assert!(r.contains(UVec2::new(14, 10)));
        assert!(r.contains(UVec2::new(10, 22)));
        assert!(r.contains(UVec2::new(6, 22)));
        assert!(r.contains(UVec2::new(14, 22)));
        assert!(!r.contains(UVec2::new(50, 5)));
    }

    #[test]
    fn rect_union() {
        let r = URect::from_center_size(UVec2::splat(4), UVec2::splat(4)); // [2, 2] - [6, 6]

        // overlapping
        let r2 = URect {
            min: UVec2::new(0, 0),
            max: UVec2::new(3, 3),
        };
        let u = r.union(r2);
        assert_eq!(u.min, UVec2::new(0, 0));
        assert_eq!(u.max, UVec2::new(6, 6));

        // disjoint
        let r2 = URect {
            min: UVec2::new(4, 7),
            max: UVec2::new(8, 8),
        };
        let u = r.union(r2);
        assert_eq!(u.min, UVec2::new(2, 2));
        assert_eq!(u.max, UVec2::new(8, 8));

        // included
        let r2 = URect::from_center_size(UVec2::splat(4), UVec2::splat(2));
        let u = r.union(r2);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);

        // including
        let r2 = URect::from_center_size(UVec2::splat(4), UVec2::splat(6));
        let u = r.union(r2);
        assert_eq!(u.min, r2.min);
        assert_eq!(u.min, r2.min);
    }

    #[test]
    fn rect_union_pt() {
        let r = URect::from_center_size(UVec2::splat(4), UVec2::splat(4)); // [2, 2] - [6, 6]

        // inside
        let v = UVec2::new(2, 5);
        let u = r.union_point(v);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);

        // outside
        let v = UVec2::new(10, 5);
        let u = r.union_point(v);
        assert_eq!(u.min, UVec2::new(2, 2));
        assert_eq!(u.max, UVec2::new(10, 6));
    }

    #[test]
    fn rect_intersect() {
        let r = URect::from_center_size(UVec2::splat(6), UVec2::splat(8)); // [2, 2] - [10, 10]

        // overlapping
        let r2 = URect {
            min: UVec2::new(8, 8),
            max: UVec2::new(12, 12),
        };
        let u = r.intersect(r2);
        assert_eq!(u.min, UVec2::new(8, 8));
        assert_eq!(u.max, UVec2::new(10, 10));

        // disjoint
        let r2 = URect {
            min: UVec2::new(12, 12),
            max: UVec2::new(14, 18),
        };
        let u = r.intersect(r2);
        assert!(u.is_empty());
        assert_eq!(u.width(), 0);

        // included
        let r2 = URect::from_center_size(UVec2::splat(6), UVec2::splat(2));
        let u = r.intersect(r2);
        assert_eq!(u.min, r2.min);
        assert_eq!(u.max, r2.max);

        // including
        let r2 = URect::from_center_size(UVec2::splat(6), UVec2::splat(10));
        let u = r.intersect(r2);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);
    }

    #[test]
    fn rect_inflate() {
        let r = URect::from_center_size(UVec2::splat(6), UVec2::splat(6)); // [3, 3] - [9, 9]

        let r2 = r.inflate(2);
        assert_eq!(r2.min, UVec2::new(1, 1));
        assert_eq!(r2.max, UVec2::new(11, 11));
    }
}
