use core::fmt::Display;

use bevy_math::IVec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// The set of possible tile orientations.
/// These represent all possible results of mirroring the tile horizontally
/// and/or vertically, and/or rotation by 90 degree increments.
///
/// Rotation is measured counter-clockwise as elsewhere in Bevy.
///
/// The representation is a u8 value where the bits (from most significant to least
/// significant), represent:
///
/// - Bit 2: Mirroring the tile horizontally (left and right sides are swapped)
/// - Bit 1: Mirroring the tile vertically (top and bottom are swapped)
/// - Bit 0: Mirroring the tile diagonally (top-right and bottom-left corners are swapped)
///
/// The order in which the mirroring is performed matters - the tile is first mirrored
/// horizontally (if specified), then vertically, then finally diagonally, in the UV
/// coordinate system.
///
/// Note that different coordinate systems are used for UV mapping and Bevy world
/// coordinates (e.g. in UV maps, the y component increases towards the bottom of
/// the tile, in Bevy it increases towards the top). This means that the ordering
/// in which the mirroring is applied differs in different coordinate systems.
///
/// The ordering of the enum starts from the default orientation (no rotation or mirroring),
/// then we have successive 90 degree counter-clockwise rotations. Then we have a tile
/// mirrored horizontally, and then the results of successive 90 degree counter-clockwise
/// rotations of the mirrored tile.
///
/// The enum values can be easily converted to the format used for tile indices in [Tiled](https://www.mapeditor.org/) maps,
/// where bits 31, 30 and 29 correspond to bits 2, 1 and 0 of this enum's values.
/// So for a given enum value, we can just use `value << 29` to produce the bits required
/// in a Tiled index.
#[derive(Debug, Clone, Copy, Reflect, Default, PartialEq, Eq, Hash)]
#[reflect(Default, Clone, PartialEq, Hash)]
#[repr(u8)]
pub enum TileOrientation {
    #[default]
    Default = 0b000,
    Rotate90 = 0b011,
    Rotate180 = 0b110,
    Rotate270 = 0b101,
    MirrorH = 0b100,
    MirrorHRotate90 = 0b001,
    MirrorHRotate180 = 0b010,
    MirrorHRotate270 = 0b111,
}

impl TileOrientation {
    /// This bit is set in the enum value if the tile is mirrored horizontally
    const MIRROR_H_BIT: u8 = 0b100;

    /// This bit is set in the enum value if the tile is mirrored vertically
    const MIRROR_V_BIT: u8 = 0b010;

    /// This bit is set in the enum value if the tile is mirrored diagonally
    const MIRROR_D_BIT: u8 = 0b001;

    /// Create a [`TileOrientation`] based on whether each mirror is applied
    pub fn from_bools(mirror_h: bool, mirror_v: bool, mirror_d: bool) -> TileOrientation {
        match (mirror_h, mirror_v, mirror_d) {
            (false, false, false) => TileOrientation::Default,
            (false, true, true) => TileOrientation::Rotate90,
            (true, true, false) => TileOrientation::Rotate180,
            (true, false, true) => TileOrientation::Rotate270,
            (true, false, false) => TileOrientation::MirrorH,
            (false, false, true) => TileOrientation::MirrorHRotate90,
            (false, true, false) => TileOrientation::MirrorHRotate180,
            (true, true, true) => TileOrientation::MirrorHRotate270,
        }
    }

    /// True if the tile is mirrored horizontally
    pub fn mirror_h(&self) -> bool {
        (*self as u8) & Self::MIRROR_H_BIT != 0
    }

    /// True if the tile is mirrored vertically
    pub fn mirror_v(&self) -> bool {
        (*self as u8) & Self::MIRROR_V_BIT != 0
    }

    /// True if the tile is mirrored diagonally
    pub fn mirror_d(&self) -> bool {
        (*self as u8) & Self::MIRROR_D_BIT != 0
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
            Self::MirrorH => Self::MirrorH,
            Self::MirrorHRotate90 => Self::MirrorHRotate90,
            Self::MirrorHRotate180 => Self::MirrorHRotate180,
            Self::MirrorHRotate270 => Self::MirrorHRotate270,
        }
    }

    /// This method treats the [`TileOrientation`] as the transform from
    /// [`TileOrientation::Default`] to this orientation.
    /// Apply this transformation to an [`IVec2`]
    pub fn apply_to_ivec2(&self, pos: &IVec2) -> IVec2 {
        let mut x = pos.x;
        let mut y = pos.y;

        // Convert to y-down coords (as per UV, Tiled)
        y = -y;

        if self.mirror_d() {
            (x, y) = (y, x);
        }
        if self.mirror_h() {
            x = -x;
        }
        if self.mirror_v() {
            y = -y;
        }

        // And back to y-up coords for Bevy
        y = -y;

        IVec2::new(x, y)
    }

    /// This method treats each [`TileOrientation`] as the transform from
    /// [`TileOrientation::Default`] to that orientation.
    /// Produce a [`TileOrientation`] that will give the same effect as
    /// applying this transform, then applying `then`.
    pub fn and_then(&self, then: TileOrientation) -> TileOrientation {
        let mut mirror_h = self.mirror_h();
        let mut mirror_v = self.mirror_v();
        let mut mirror_d = self.mirror_d();

        if then.mirror_d() {
            mirror_d = !mirror_d;
            (mirror_h, mirror_v) = (mirror_v, mirror_h);
        }
        if then.mirror_h() {
            mirror_h = !mirror_h;
        }
        if then.mirror_v() {
            mirror_v = !mirror_v;
        }

        Self::from_bools(mirror_h, mirror_v, mirror_d)
    }
}

impl Display for TileOrientation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => write!(f, "Default"),
            Self::Rotate90 => write!(f, "Rotate90"),
            Self::Rotate180 => write!(f, "Rotate180"),
            Self::Rotate270 => write!(f, "Rotate270"),
            Self::MirrorH => write!(f, "MirrorX"),
            Self::MirrorHRotate90 => write!(f, "MirrorXRotate90"),
            Self::MirrorHRotate180 => write!(f, "MirrorXRotate180"),
            Self::MirrorHRotate270 => write!(f, "MirrorXRotate270"),
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_math::ivec2;

    use super::*;

    const CASES: [(TileOrientation, bool, bool, bool); 8] = [
        (TileOrientation::Default, false, false, false),
        (TileOrientation::Rotate90, false, true, true),
        (TileOrientation::Rotate180, true, true, false),
        (TileOrientation::Rotate270, true, false, true),
        (TileOrientation::MirrorH, true, false, false),
        (TileOrientation::MirrorHRotate90, false, false, true),
        (TileOrientation::MirrorHRotate180, false, true, false),
        (TileOrientation::MirrorHRotate270, true, true, true),
    ];

    #[test]
    fn mirror_and_swap_bits_should_extract_correctly() {
        for (orientation, mirror_h, mirror_v, mirror_d) in CASES.iter() {
            assert_eq!(orientation.mirror_h(), *mirror_h);
            assert_eq!(orientation.mirror_v(), *mirror_v);
            assert_eq!(orientation.mirror_d(), *mirror_d);
        }
    }

    #[test]
    fn from_bools_should_give_correct_orientation() {
        for (orientation, mirror_h, mirror_v, mirror_d) in CASES.iter() {
            let from_bools = TileOrientation::from_bools(*mirror_h, *mirror_v, *mirror_d);

            // Check against cases, but this is a bit duplicative of the code itself
            assert_eq!(*orientation, from_bools);

            // Now check we get the right bits in the u8, as a better test
            let from_bools_u8 = from_bools as u8;
            assert_eq!(
                from_bools_u8 & TileOrientation::MIRROR_H_BIT != 0,
                *mirror_h
            );
            assert_eq!(
                from_bools_u8 & TileOrientation::MIRROR_V_BIT != 0,
                *mirror_v
            );
            assert_eq!(
                from_bools_u8 & TileOrientation::MIRROR_D_BIT != 0,
                *mirror_d
            );
        }
    }

    #[test]
    fn applying_transform_and_then_inverse_transform_should_yield_default() {
        for (orientation, ..) in CASES.iter() {
            let inverse = orientation.inverse();
            let transform_then_inverse = orientation.and_then(inverse);
            assert_eq!(transform_then_inverse, TileOrientation::Default);
        }
    }

    // The "from" point for position test cases, using a point without any
    // symmetry for mirroring horizontally or vertically, or 90 degree rotations about the origin.
    const FROM_POS: IVec2 = ivec2(1, 2);

    // Each case shows where FROM_POS maps to, under the given transform
    // Worked out by hand with paper :)
    const POS_CASES: [(TileOrientation, IVec2); 8] = [
        (TileOrientation::Default, ivec2(1, 2)),
        (TileOrientation::Rotate90, ivec2(-2, 1)),
        (TileOrientation::Rotate180, ivec2(-1, -2)),
        (TileOrientation::Rotate270, ivec2(2, -1)),
        (TileOrientation::MirrorH, ivec2(-1, 2)),
        (TileOrientation::MirrorHRotate90, ivec2(-2, -1)),
        (TileOrientation::MirrorHRotate180, ivec2(1, -2)),
        (TileOrientation::MirrorHRotate270, ivec2(2, 1)),
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
