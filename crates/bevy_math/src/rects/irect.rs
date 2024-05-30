use crate::{IVec2, Rect, URect};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A rectangle defined by two opposite corners.
///
/// The rectangle is axis aligned, and defined by its minimum and maximum coordinates,
/// stored in `IRect::min` and `IRect::max`, respectively. The minimum/maximum invariant
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
pub struct IRect {
    /// The minimum corner point of the rect.
    pub min: IVec2,
    /// The maximum corner point of the rect.
    pub max: IVec2,
}

impl IRect {
    /// An empty `IRect`, represented by maximum and minimum corner points
    /// with `max == IVec2::MIN` and `min == IVec2::MAX`, so the
    /// rect has an extremely large negative size.
    /// This is useful, because when taking a union B of a non-empty `IRect` A and
    /// this empty `IRect`, B will simply equal A.
    pub const EMPTY: Self = Self {
        max: IVec2::MIN,
        min: IVec2::MAX,
    };
    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::IRect;
    /// let r = IRect::new(0, 4, 10, 6); // w=10 h=2
    /// let r = IRect::new(2, 3, 5, -1); // w=3 h=4
    /// ```
    #[inline]
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self::from_corners(IVec2::new(x0, y0), IVec2::new(x1, y1))
    }

    /// Create a new rectangle from two corner points.
    ///
    /// The two points do not need to be the minimum and/or maximum corners.
    /// They only need to be two opposite corners.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// // Unit rect from [0,0] to [1,1]
    /// let r = IRect::from_corners(IVec2::ZERO, IVec2::ONE); // w=1 h=1
    /// // Same; the points do not need to be ordered
    /// let r = IRect::from_corners(IVec2::ONE, IVec2::ZERO); // w=1 h=1
    /// ```
    #[inline]
    pub fn from_corners(p0: IVec2, p1: IVec2) -> Self {
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
    /// This method panics if any of the components of the size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::from_center_size(IVec2::ZERO, IVec2::new(3, 2)); // w=2 h=2
    /// assert_eq!(r.min, IVec2::splat(-1));
    /// assert_eq!(r.max, IVec2::splat(1));
    /// ```
    #[inline]
    pub fn from_center_size(origin: IVec2, size: IVec2) -> Self {
        debug_assert!(size.cmpge(IVec2::ZERO).all(), "IRect size must be positive");
        let half_size = size / 2;
        Self::from_center_half_size(origin, half_size)
    }

    /// Create a new rectangle from its center and half-size.
    ///
    /// # Panics
    ///
    /// This method panics if any of the components of the half-size is negative.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::from_center_half_size(IVec2::ZERO, IVec2::ONE); // w=2 h=2
    /// assert_eq!(r.min, IVec2::splat(-1));
    /// assert_eq!(r.max, IVec2::splat(1));
    /// ```
    #[inline]
    pub fn from_center_half_size(origin: IVec2, half_size: IVec2) -> Self {
        assert!(
            half_size.cmpge(IVec2::ZERO).all(),
            "IRect half_size must be positive"
        );
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
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::from_corners(IVec2::ZERO, IVec2::new(0, 1)); // w=0 h=1
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
    /// # use bevy_math::IRect;
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.width(), 5);
    /// ```
    #[inline]
    pub fn width(&self) -> i32 {
        self.max.x - self.min.x
    }

    /// Rectangle height (max.y - min.y).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::IRect;
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.height(), 1);
    /// ```
    #[inline]
    pub fn height(&self) -> i32 {
        self.max.y - self.min.y
    }

    /// Rectangle size.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// assert_eq!(r.size(), IVec2::new(5, 1));
    /// ```
    #[inline]
    pub fn size(&self) -> IVec2 {
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
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::new(0, 0, 4, 3); // w=4 h=3
    /// assert_eq!(r.half_size(), IVec2::new(2, 1));
    /// ```
    #[inline]
    pub fn half_size(&self) -> IVec2 {
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
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::new(0, 0, 5, 2); // w=5 h=2
    /// assert_eq!(r.center(), IVec2::new(2, 1));
    /// ```
    #[inline]
    pub fn center(&self) -> IVec2 {
        (self.min + self.max) / 2
    }

    /// Check if a point lies within this rectangle, inclusive of its edges.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::IRect;
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// assert!(r.contains(r.center()));
    /// assert!(r.contains(r.min));
    /// assert!(r.contains(r.max));
    /// ```
    #[inline]
    pub fn contains(&self, point: IVec2) -> bool {
        (point.cmpge(self.min) & point.cmple(self.max)).all()
    }

    /// Build a new rectangle formed of the union of this rectangle and another rectangle.
    ///
    /// The union is the smallest rectangle enclosing both rectangles.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r1 = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// let r2 = IRect::new(1, -1, 3, 3); // w=2 h=4
    /// let r = r1.union(r2);
    /// assert_eq!(r.min, IVec2::new(0, -1));
    /// assert_eq!(r.max, IVec2::new(5, 3));
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
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// let u = r.union_point(IVec2::new(3, 6));
    /// assert_eq!(u.min, IVec2::ZERO);
    /// assert_eq!(u.max, IVec2::new(5, 6));
    /// ```
    #[inline]
    pub fn union_point(&self, other: IVec2) -> Self {
        Self {
            min: self.min.min(other),
            max: self.max.max(other),
        }
    }

    /// Build a new rectangle formed of the intersection of this rectangle and another rectangle.
    ///
    /// The intersection is the largest rectangle enclosed in both rectangles. If the intersection
    /// is empty, this method returns an empty rectangle ([`IRect::is_empty()`] returns `true`), but
    /// the actual values of [`IRect::min`] and [`IRect::max`] are implementation-dependent.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r1 = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// let r2 = IRect::new(1, -1, 3, 3); // w=2 h=4
    /// let r = r1.intersect(r2);
    /// assert_eq!(r.min, IVec2::new(1, 0));
    /// assert_eq!(r.max, IVec2::new(3, 1));
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
    /// If this would result in zero or negative width or height, [`IRect::EMPTY`] is returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_math::{IRect, IVec2};
    /// let r = IRect::new(0, 0, 5, 1); // w=5 h=1
    /// let r2 = r.inflate(3); // w=11 h=7
    /// assert_eq!(r2.min, IVec2::splat(-3));
    /// assert_eq!(r2.max, IVec2::new(8, 4));
    ///
    /// let r = IRect::new(0, -1, 4, 3); // w=4 h=4
    /// let r2 = r.inflate(-1); // w=2 h=2
    /// assert_eq!(r2.min, IVec2::new(1, 0));
    /// assert_eq!(r2.max, IVec2::new(3, 2));
    /// ```
    #[inline]
    pub fn inflate(&self, expansion: i32) -> Self {
        let mut r = Self {
            min: self.min - expansion,
            max: self.max + expansion,
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

    /// Returns self as [`URect`] (u32)
    #[inline]
    pub fn as_urect(&self) -> URect {
        URect::from_corners(self.min.as_uvec2(), self.max.as_uvec2())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn well_formed() {
        let r = IRect::from_center_size(IVec2::new(3, -5), IVec2::new(8, 12));

        assert_eq!(r.min, IVec2::new(-1, -11));
        assert_eq!(r.max, IVec2::new(7, 1));

        assert_eq!(r.center(), IVec2::new(3, -5));

        assert_eq!(r.width().abs(), 8);
        assert_eq!(r.height().abs(), 12);
        assert_eq!(r.size(), IVec2::new(8, 12));
        assert_eq!(r.half_size(), IVec2::new(4, 6));

        assert!(r.contains(IVec2::new(3, -5)));
        assert!(r.contains(IVec2::new(-1, -10)));
        assert!(r.contains(IVec2::new(-1, 0)));
        assert!(r.contains(IVec2::new(7, -10)));
        assert!(r.contains(IVec2::new(7, 0)));
        assert!(!r.contains(IVec2::new(50, -5)));
    }

    #[test]
    fn rect_union() {
        let r = IRect::from_center_size(IVec2::ZERO, IVec2::splat(4)); // [-2, -2] - [2, 2]

        // overlapping
        let r2 = IRect {
            min: IVec2::new(1, 1),
            max: IVec2::new(3, 3),
        };
        let u = r.union(r2);
        assert_eq!(u.min, IVec2::new(-2, -2));
        assert_eq!(u.max, IVec2::new(3, 3));

        // disjoint
        let r2 = IRect {
            min: IVec2::new(1, 4),
            max: IVec2::new(4, 6),
        };
        let u = r.union(r2);
        assert_eq!(u.min, IVec2::new(-2, -2));
        assert_eq!(u.max, IVec2::new(4, 6));

        // included
        let r2 = IRect::from_center_size(IVec2::ZERO, IVec2::splat(2));
        let u = r.union(r2);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);

        // including
        let r2 = IRect::from_center_size(IVec2::ZERO, IVec2::splat(6));
        let u = r.union(r2);
        assert_eq!(u.min, r2.min);
        assert_eq!(u.min, r2.min);
    }

    #[test]
    fn rect_union_pt() {
        let r = IRect::from_center_size(IVec2::ZERO, IVec2::splat(4)); // [-2,-2] - [2,2]

        // inside
        let v = IVec2::new(1, -1);
        let u = r.union_point(v);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);

        // outside
        let v = IVec2::new(10, -3);
        let u = r.union_point(v);
        assert_eq!(u.min, IVec2::new(-2, -3));
        assert_eq!(u.max, IVec2::new(10, 2));
    }

    #[test]
    fn rect_intersect() {
        let r = IRect::from_center_size(IVec2::ZERO, IVec2::splat(8)); // [-4,-4] - [4,4]

        // overlapping
        let r2 = IRect {
            min: IVec2::new(2, 2),
            max: IVec2::new(6, 6),
        };
        let u = r.intersect(r2);
        assert_eq!(u.min, IVec2::new(2, 2));
        assert_eq!(u.max, IVec2::new(4, 4));

        // disjoint
        let r2 = IRect {
            min: IVec2::new(-8, -2),
            max: IVec2::new(-6, 2),
        };
        let u = r.intersect(r2);
        assert!(u.is_empty());
        assert_eq!(u.width(), 0);

        // included
        let r2 = IRect::from_center_size(IVec2::ZERO, IVec2::splat(2));
        let u = r.intersect(r2);
        assert_eq!(u.min, r2.min);
        assert_eq!(u.max, r2.max);

        // including
        let r2 = IRect::from_center_size(IVec2::ZERO, IVec2::splat(10));
        let u = r.intersect(r2);
        assert_eq!(u.min, r.min);
        assert_eq!(u.max, r.max);
    }

    #[test]
    fn rect_inflate() {
        let r = IRect::from_center_size(IVec2::ZERO, IVec2::splat(4)); // [-2,-2] - [2,2]

        let r2 = r.inflate(2);
        assert_eq!(r2.min, IVec2::new(-4, -4));
        assert_eq!(r2.max, IVec2::new(4, 4));
    }
}
