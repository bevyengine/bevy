use core::fmt::Display;

use bevy_math::IVec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The set of possible tile orientations based on any combination of mirroring in
/// x and/or y axes, and/or rotation by 90 degree increments.
/// The ordering starts from the default orientation (no rotation or mirroring),
/// then we have 90 degree clockwise rotations, then we start from a mirror in the x
/// and perform 90 degree rotations of that.
/// The representation is a u8 value where the bits (from most significant to least
/// significant), represent mirroring in the x axis, then the y axis, then the diagonal
/// axis running from the top-left of the tile to the bottom-right (which corresponds to
/// swapping the x and y axes).
/// This allows easy conversion to the format used for tile indices in Tiled maps,
/// where bits 31, 30 and 29 correspond to bits 2, 1 and 0 of this enum's values.
/// So for a given enum value, we can just use `value << 29` to produce the bits required
/// in a Tiled index.
#[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq, Hash)]
#[reflect(Default, Clone, PartialEq, Hash)]
#[repr(u8)]
pub enum TileOrientation {
    #[default]
    Default = 0b000,
    Rotate90 = 0b101,
    Rotate180 = 0b110,
    Rotate270 = 0b011,
    MirrorX = 0b100,
    MirrorXRotate90 = 0b111,
    MirrorXRotate180 = 0b010,
    MirrorXRotate270 = 0b001,
}

impl TileOrientation {
    /// This bit is set in the enum value if the tile is mirrored in the x axis
    const MIRROR_X_BIT: u8 = 0b100;

    /// This bit is set in the enum value if the tile is mirrored in the y axis
    const MIRROR_Y_BIT: u8 = 0b010;

    /// This bit is set in the enum value if the tile has the x and y axes swapped
    const SWAP_XY_BIT: u8 = 0b001;

    /// Create a [`TileOrientation`] based on whether each component mirror/swap is applied
    pub fn from_bools(mirror_x: bool, mirror_y: bool, swap_xy: bool) -> TileOrientation {
        match (mirror_x, mirror_y, swap_xy) {
            (false, false, false) => TileOrientation::Default,
            (true, false, true) => TileOrientation::Rotate90,
            (true, true, false) => TileOrientation::Rotate180,
            (false, true, true) => TileOrientation::Rotate270,
            (true, false, false) => TileOrientation::MirrorX,
            (true, true, true) => TileOrientation::MirrorXRotate90,
            (false, true, false) => TileOrientation::MirrorXRotate180,
            (false, false, true) => TileOrientation::MirrorXRotate270,
        }
    }

    /// True if the tile is mirrored in the x axis
    pub fn mirror_x(&self) -> bool {
        (*self as u8) & Self::MIRROR_X_BIT != 0
    }

    /// True if the tile is mirrored in the y axis
    pub fn mirror_y(&self) -> bool {
        (*self as u8) & Self::MIRROR_Y_BIT != 0
    }

    /// True if the tile has the x and y axes swapped
    pub fn swap_xy(&self) -> bool {
        (*self as u8) & Self::SWAP_XY_BIT != 0
    }

    /// This method treats each [`TileOrientation`] as the transform from
    /// [`TileOrientation::Default`] to that orientation.
    /// Find the [`TileOrientation`] that when applied will undo the effect
    /// of this [`TileOrientation`]
    pub fn inverse(&self) -> TileOrientation {
        match self {
            Self::Default => Self::Default,
            Self::Rotate90 => Self::Rotate270,
            Self::Rotate180 => Self::Rotate180,
            Self::Rotate270 => Self::Rotate90,
            Self::MirrorX => Self::MirrorX,
            Self::MirrorXRotate90 => Self::MirrorXRotate90,
            Self::MirrorXRotate180 => Self::MirrorXRotate180,
            Self::MirrorXRotate270 => Self::MirrorXRotate270,
        }
    }

    /// This method treats the [`TileOrientation`] as the transform from
    /// [`TileOrientation::Default`] to this orientation.
    /// Apply this transformation to an [`IVec2`]
    pub fn apply_to_ivec2(&self, pos: &IVec2) -> IVec2 {
        let mut x = pos.x;
        let mut y = pos.y;
        if self.swap_xy() {
            (x, y) = (y, x);
        }
        if self.mirror_x() {
            x = -x;
        }
        if self.mirror_y() {
            y = -y;
        }
        IVec2::new(x, y)
    }

    /// This method treats each [`TileOrientation`] as the transform from
    /// [`TileOrientation::Default`] to that orientation.
    /// Produce a [`TileOrientation`] that will give the same effect as
    /// applying this transform, then applying `then`.
    pub fn and_then(&self, then: TileOrientation) -> TileOrientation {
        let mut mirror_x = self.mirror_x();
        let mut mirror_y = self.mirror_y();
        let mut swap_xy = self.swap_xy();

        if then.swap_xy() {
            swap_xy = !swap_xy;
            (mirror_x, mirror_y) = (mirror_y, mirror_x);
        }
        if then.mirror_x() {
            mirror_x = !mirror_x;
        }
        if then.mirror_y() {
            mirror_y = !mirror_y;
        }

        Self::from_bools(mirror_x, mirror_y, swap_xy)
    }
}

impl Display for TileOrientation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => write!(f, "Default"),
            Self::Rotate90 => write!(f, "Rotate90"),
            Self::Rotate180 => write!(f, "Rotate180"),
            Self::Rotate270 => write!(f, "Rotate270"),
            Self::MirrorX => write!(f, "MirrorX"),
            Self::MirrorXRotate90 => write!(f, "MirrorXRotate90"),
            Self::MirrorXRotate180 => write!(f, "MirrorXRotate180"),
            Self::MirrorXRotate270 => write!(f, "MirrorXRotate270"),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::ivec2;

    use super::*;

    const CASES: [(TileOrientation, bool, bool, bool); 8] = [
        (TileOrientation::Default, false, false, false),
        (TileOrientation::Rotate90, true, false, true),
        (TileOrientation::Rotate180, true, true, false),
        (TileOrientation::Rotate270, false, true, true),
        (TileOrientation::MirrorX, true, false, false),
        (TileOrientation::MirrorXRotate90, true, true, true),
        (TileOrientation::MirrorXRotate180, false, true, false),
        (TileOrientation::MirrorXRotate270, false, false, true),
    ];

    #[test]
    fn mirror_and_swap_bits_should_extract_correctly() {
        for (orientation, mirror_x, mirror_y, swap_xy) in CASES.iter() {
            assert_eq!(orientation.mirror_x(), *mirror_x);
            assert_eq!(orientation.mirror_y(), *mirror_y);
            assert_eq!(orientation.swap_xy(), *swap_xy);
        }
    }

    #[test]
    fn from_bools_should_give_correct_orientation() {
        for (orientation, mirror_x, mirror_y, swap_xy) in CASES.iter() {
            let from_bools = TileOrientation::from_bools(*mirror_x, *mirror_y, *swap_xy);

            // Check against cases, but this is a bit duplicative of the code itself
            assert_eq!(*orientation, from_bools);

            // Now check we get the right bits in the u8, as a better test
            let from_bools_u8 = from_bools as u8;
            assert_eq!(
                from_bools_u8 & TileOrientation::MIRROR_X_BIT != 0,
                *mirror_x
            );
            assert_eq!(
                from_bools_u8 & TileOrientation::MIRROR_Y_BIT != 0,
                *mirror_y
            );
            assert_eq!(from_bools_u8 & TileOrientation::SWAP_XY_BIT != 0, *swap_xy);
        }
    }

    // The "from" point for position test cases, using a point without any
    // symmetry for mirroring in x or y, or 90 degree rotations about the origin.
    const FROM_POS: IVec2 = ivec2(1, 2);

    // Each case shows where FROM_POS maps to, under the given transform
    // Worked out by hand with paper :)
    const POS_CASES: [(TileOrientation, IVec2); 8] = [
        (TileOrientation::Default, ivec2(1, 2)),
        (TileOrientation::Rotate90, ivec2(-2, 1)),
        (TileOrientation::Rotate180, ivec2(-1, -2)),
        (TileOrientation::Rotate270, ivec2(2, -1)),
        (TileOrientation::MirrorX, ivec2(-1, 2)),
        (TileOrientation::MirrorXRotate90, ivec2(-2, -1)),
        (TileOrientation::MirrorXRotate180, ivec2(1, -2)),
        (TileOrientation::MirrorXRotate270, ivec2(2, 1)),
    ];

    #[test]
    fn applying_to_pos_should_give_correct_new_pos() {
        for (orientation, end_pos) in POS_CASES.iter() {
            let transform_end_pos = orientation.apply_to_ivec2(&FROM_POS);
            assert_eq!(
                end_pos, &transform_end_pos,
                "{:?} should map {} to {}, but we got {}",
                orientation, FROM_POS, end_pos, transform_end_pos
            );
        }
    }

    #[test]
    fn applying_transform_then_inverse_should_leave_pos_unaltered() {
        for (orientation, ..) in CASES.iter() {
            let transformed = orientation.apply_to_ivec2(&FROM_POS);
            let transformed_back = orientation.inverse().apply_to_ivec2(&transformed);
            assert_eq!(FROM_POS, transformed_back);
        }
    }

    #[test]
    fn applying_inverse_then_transform_should_leave_pos_unaltered() {
        for (orientation, ..) in CASES.iter() {
            let inverse_transformed = orientation.inverse().apply_to_ivec2(&FROM_POS);
            let transformed_back = orientation.apply_to_ivec2(&inverse_transformed);
            assert_eq!(FROM_POS, transformed_back);
        }
    }

    #[test]
    fn applying_any_transform_pair_individually_to_a_pos_should_give_same_result_as_applying_combined_single_transform(
    ) {
        for (first, ..) in CASES.iter() {
            for (second, ..) in CASES.iter() {
                let combined = first.and_then(*second);
                assert_eq!(
                    second.apply_to_ivec2(&first.apply_to_ivec2(&FROM_POS)),
                    combined.apply_to_ivec2(&FROM_POS)
                );
            }
        }
    }
}
