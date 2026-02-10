use core::fmt::Display;

use bevy_math::IVec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The set of possible tile transforms based on any combination of mirroring in
/// x and/or y axes, and/or rotation by 90 degree increments.
/// The ordering starts from no transformation, then we have 90 degree clockwise rotations,
/// then we start from a mirror in the x and perform 90 degree rotations of that.
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
pub enum TileTransform {
    #[default]
    None = 0b000,
    Rotate90 = 0b101,
    Rotate180 = 0b110,
    Rotate270 = 0b011,
    MirrorX = 0b100,
    MirrorXRotate90 = 0b111,
    MirrorXRotate180 = 0b010,
    MirrorXRotate270 = 0b001,
}

impl TileTransform {
    /// This bit is set in the enum value if the transformation includes mirroring in the x axis
    const MIRROR_X_BIT: u8 = 0b100;

    /// This bit is set in the enum value if the transformation includes mirroring in the y axis
    const MIRROR_Y_BIT: u8 = 0b010;

    /// This bit is set in the enum value if the transformation includes swapping the x and y axes
    const SWAP_XY_BIT: u8 = 0b001;

    /// Create a [`TileTransform`] based on whether each component mirror is applied
    pub fn from_bools(mirror_x: bool, mirror_y: bool, swap_xy: bool) -> TileTransform {
        match (mirror_x, mirror_y, swap_xy) {
            (false, false, false) => TileTransform::None,
            (true, false, true) => TileTransform::Rotate90,
            (true, true, false) => TileTransform::Rotate180,
            (false, true, true) => TileTransform::Rotate270,
            (true, false, false) => TileTransform::MirrorX,
            (true, true, true) => TileTransform::MirrorXRotate90,
            (false, true, false) => TileTransform::MirrorXRotate180,
            (false, false, true) => TileTransform::MirrorXRotate270,
        }
    }

    /// True if the transformation includes mirroring in the x axis
    pub fn mirror_x(&self) -> bool {
        (*self as u8) & Self::MIRROR_X_BIT != 0
    }

    /// True if the transformation includes mirroring in the y axis
    pub fn mirror_y(&self) -> bool {
        (*self as u8) & Self::MIRROR_Y_BIT != 0
    }

    /// True if the transformation includes swapping the x and y axes
    pub fn swap_xy(&self) -> bool {
        (*self as u8) & Self::SWAP_XY_BIT != 0
    }

    /// Find the [`TileTransform`] that when applied will undo the effect
    /// of this [`TileTransform`]
    pub fn inverse(&self) -> TileTransform {
        match self {
            Self::None => Self::None,
            Self::Rotate90 => Self::Rotate270,
            Self::Rotate180 => Self::Rotate180,
            Self::Rotate270 => Self::Rotate90,
            Self::MirrorX => Self::MirrorX,
            Self::MirrorXRotate90 => Self::MirrorXRotate90,
            Self::MirrorXRotate180 => Self::MirrorXRotate180,
            Self::MirrorXRotate270 => Self::MirrorXRotate270,
        }
    }

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

    /// Produce a [`TileTransform`] that will give the same effect as
    /// applying this transform, then applying `then`.
    pub fn and_then(&self, then: TileTransform) -> TileTransform {
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

impl Display for TileTransform {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::None => write!(f, "None"),
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

    const CASES: [(TileTransform, bool, bool, bool); 8] = [
        (TileTransform::None, false, false, false),
        (TileTransform::Rotate90, true, false, true),
        (TileTransform::Rotate180, true, true, false),
        (TileTransform::Rotate270, false, true, true),
        (TileTransform::MirrorX, true, false, false),
        (TileTransform::MirrorXRotate90, true, true, true),
        (TileTransform::MirrorXRotate180, false, true, false),
        (TileTransform::MirrorXRotate270, false, false, true),
    ];

    #[test]
    fn mirror_and_swap_bits_should_extract_correctly() {
        for (transform, mirror_x, mirror_y, swap_xy) in CASES.iter() {
            assert_eq!(transform.mirror_x(), *mirror_x);
            assert_eq!(transform.mirror_y(), *mirror_y);
            assert_eq!(transform.swap_xy(), *swap_xy);
        }
    }

    #[test]
    fn from_bools_should_give_correct_transform() {
        for (transform, mirror_x, mirror_y, swap_xy) in CASES.iter() {
            let from_bools = TileTransform::from_bools(*mirror_x, *mirror_y, *swap_xy);

            // Check against cases, but this is a bit duplicative of the code itself
            assert_eq!(*transform, from_bools);

            // Now check we get the right bits in the u8, as a better test
            let from_bools_u8 = from_bools as u8;
            assert_eq!(from_bools_u8 & TileTransform::MIRROR_X_BIT != 0, *mirror_x);
            assert_eq!(from_bools_u8 & TileTransform::MIRROR_Y_BIT != 0, *mirror_y);
            assert_eq!(from_bools_u8 & TileTransform::SWAP_XY_BIT != 0, *swap_xy);
        }
    }

    // The "from" point for position test cases, using a point without any
    // symmetry for mirroring in x or y, or 90 degree rotations about the origin.
    const FROM_POS: IVec2 = ivec2(1, 2);

    // Each case shows where FROM_POS maps to, under the given transform
    // Worked out by hand with paper :)
    const POS_CASES: [(TileTransform, IVec2); 8] = [
        (TileTransform::None, ivec2(1, 2)),
        (TileTransform::Rotate90, ivec2(-2, 1)),
        (TileTransform::Rotate180, ivec2(-1, -2)),
        (TileTransform::Rotate270, ivec2(2, -1)),
        (TileTransform::MirrorX, ivec2(-1, 2)),
        (TileTransform::MirrorXRotate90, ivec2(-2, -1)),
        (TileTransform::MirrorXRotate180, ivec2(1, -2)),
        (TileTransform::MirrorXRotate270, ivec2(2, 1)),
    ];

    #[test]
    fn applying_to_pos_should_give_correct_new_pos() {
        for (transform, end_pos) in POS_CASES.iter() {
            let transform_end_pos = transform.apply_to_ivec2(&FROM_POS);
            assert_eq!(
                end_pos, &transform_end_pos,
                "{:?} should map {} to {}, but we got {}",
                transform, FROM_POS, end_pos, transform_end_pos
            );
        }
    }

    #[test]
    fn applying_transform_then_inverse_should_leave_pos_unaltered() {
        for (transform, ..) in CASES.iter() {
            let transformed = transform.apply_to_ivec2(&FROM_POS);
            let transformed_back = transform.inverse().apply_to_ivec2(&transformed);
            assert_eq!(FROM_POS, transformed_back);
        }
    }

    #[test]
    fn applying_inverse_then_transform_should_leave_pos_unaltered() {
        for (transform, ..) in CASES.iter() {
            let inverse_transformed = transform.inverse().apply_to_ivec2(&FROM_POS);
            let transformed_back = transform.apply_to_ivec2(&inverse_transformed);
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
