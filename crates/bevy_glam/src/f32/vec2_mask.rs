use super::Vec2;
use core::{fmt, ops::*};

/// A 2-dimensional vector mask.
///
/// This type is typically created by comparison methods on `Vec2`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(C)]
pub struct Vec2Mask(u32, u32);

impl Vec2Mask {
    /// Creates a new `Vec2Mask`.
    #[inline]
    pub fn new(x: bool, y: bool) -> Self {
        const MASK: [u32; 2] = [0, 0xff_ff_ff_ff];
        Self(MASK[x as usize], MASK[y as usize])
    }

    /// Returns a bitmask with the lowest two bits set from the elements of
    /// the `Vec2Mask`.
    ///
    /// A true element results in a `1` bit and a false element in a `0` bit.
    /// Element `x` goes into the first lowest bit, element `y` into the
    /// second, etc.
    #[inline]
    pub fn bitmask(self) -> u32 {
        (self.0 & 0x1) | (self.1 & 0x1) << 1
    }

    /// Returns true if any of the elements are true, false otherwise.
    ///
    /// In other words: `x || y`.
    #[inline]
    pub fn any(self) -> bool {
        // implementaton matches SSE2 `Vec4Mask` version
        ((self.0 | self.1) & 0x1) != 0
    }

    /// Returns true if all the elements are true, false otherwise.
    ///
    /// In other words: `x && y`.
    #[inline]
    pub fn all(self) -> bool {
        // implementaton matches SSE2 `Vec4Mask` version
        ((self.0 & self.1) & 0x1) != 0
    }

    /// Creates a new `Vec2` from the elements in `if_true` and `if_false`,
    /// selecting which to use for each element based on the `Vec2Mask`.
    ///
    /// A true element in the mask uses the corresponding element from
    /// `if_true`, and false uses the element from `if_false`.
    #[inline]
    pub fn select(self, if_true: Vec2, if_false: Vec2) -> Vec2 {
        Vec2(
            if self.0 != 0 { if_true.0 } else { if_false.0 },
            if self.1 != 0 { if_true.1 } else { if_false.1 },
        )
    }
}

impl BitAnd for Vec2Mask {
    type Output = Self;
    #[inline]
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0, self.1 & other.1)
    }
}

impl BitAndAssign for Vec2Mask {
    #[inline]
    fn bitand_assign(&mut self, other: Self) {
        self.0 &= other.0;
        self.1 &= other.1;
    }
}

impl BitOr for Vec2Mask {
    type Output = Self;
    #[inline]
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0, self.1 | other.1)
    }
}

impl BitOrAssign for Vec2Mask {
    #[inline]
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
        self.1 |= other.1;
    }
}

impl Not for Vec2Mask {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        Self(!self.0, !self.1)
    }
}

impl fmt::Debug for Vec2Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Vec2Mask({:#x}, {:#x})", self.0, self.1)
    }
}

impl fmt::Display for Vec2Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.0 != 0, self.1 != 0)
    }
}

impl From<Vec2Mask> for [u32; 2] {
    #[inline]
    fn from(mask: Vec2Mask) -> Self {
        [mask.0, mask.1]
    }
}

impl AsRef<[u32; 2]> for Vec2Mask {
    #[inline]
    fn as_ref(&self) -> &[u32; 2] {
        unsafe { &*(self as *const Self as *const [u32; 2]) }
    }
}
