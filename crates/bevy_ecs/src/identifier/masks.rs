use std::num::NonZeroU32;

use super::kinds::IdKind;

/// Mask for extracting the value portion of a 32-bit high segment. This
/// yields 31-bits of total value, as the final bit (the most significant)
/// is reserved as a flag bit. Can be negated to extract the flag bit.
pub(crate) const HIGH_MASK: u32 = 0x7FFF_FFFF;

/// Abstraction over masks needed to extract values/components of an [`super::Identifier`].
pub(crate) struct IdentifierMask;

impl IdentifierMask {
    /// Returns the low component from a `u64` value
    #[inline(always)]
    pub(crate) const fn get_low(value: u64) -> u32 {
        // This will truncate to the lowest 32 bits
        value as u32
    }

    /// Returns the high component from a `u64` value
    #[inline(always)]
    pub(crate) const fn get_high(value: u64) -> u32 {
        // This will discard the lowest 32 bits
        (value >> u32::BITS) as u32
    }

    /// Pack a low and high `u32` values into a single `u64` value.
    #[inline(always)]
    pub(crate) const fn pack_into_u64(low: u32, high: u32) -> u64 {
        ((high as u64) << u32::BITS) | (low as u64)
    }

    /// Pack the [`IdKind`] bits into a high segment.
    #[inline(always)]
    pub(crate) const fn pack_kind_into_high(value: u32, kind: IdKind) -> u32 {
        value | ((kind as u32) << 24)
    }

    /// Extract the value component from a high segment of an [`super::Identifier`].
    #[inline(always)]
    pub(crate) const fn extract_value_from_high(value: u32) -> u32 {
        value & HIGH_MASK
    }

    /// Extract the ID kind component from a high segment of an [`super::Identifier`].
    #[inline(always)]
    pub(crate) const fn extract_kind_from_high(value: u32) -> IdKind {
        // The negated HIGH_MASK will extract just the bit we need for kind.
        let kind_mask = !HIGH_MASK;
        let bit = value & kind_mask;

        if bit == kind_mask {
            IdKind::Placeholder
        } else {
            IdKind::Entity
        }
    }

    /// Offsets a masked generation value by the specified amount, wrapping to 1 instead of 0.
    /// Will never be greater than [`HIGH_MASK`] or less than `1`, and increments are masked to
    /// never be greater than [`HIGH_MASK`].
    #[inline(always)]
    pub(crate) const fn inc_masked_high_by(lhs: NonZeroU32, rhs: u32) -> NonZeroU32 {
        let lo = (lhs.get() & HIGH_MASK).wrapping_add(rhs & HIGH_MASK);
        // Checks high 32 bit for whether we have overflowed 31 bits.
        let overflowed = lo >> 31;

        // SAFETY:
        // - Adding the overflow flag will offset overflows to start at 1 instead of 0
        // - The sum of `0x7FFF_FFFF` + `u32::MAX` + 1 (overflow) == `0x7FFF_FFFF`
        // - If the operation doesn't overflow at 31 bits, no offsetting takes place
        unsafe { NonZeroU32::new_unchecked(lo.wrapping_add(overflowed) & HIGH_MASK) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_u64_parts() {
        // Two distinct bit patterns per low/high component
        let value: u64 = 0x7FFF_FFFF_0000_000C;

        assert_eq!(IdentifierMask::get_low(value), 0x0000_000C);
        assert_eq!(IdentifierMask::get_high(value), 0x7FFF_FFFF);
    }

    #[test]
    fn extract_kind() {
        // All bits are ones.
        let high: u32 = 0xFFFF_FFFF;

        assert_eq!(
            IdentifierMask::extract_kind_from_high(high),
            IdKind::Placeholder
        );

        // Second and second to last bits are ones.
        let high: u32 = 0x4000_0002;

        assert_eq!(IdentifierMask::extract_kind_from_high(high), IdKind::Entity);
    }

    #[test]
    fn extract_high_value() {
        // All bits are ones.
        let high: u32 = 0xFFFF_FFFF;

        // Excludes the most significant bit as that is a flag bit.
        assert_eq!(IdentifierMask::extract_value_from_high(high), 0x7FFF_FFFF);

        // Start bit and end bit are ones.
        let high: u32 = 0x8000_0001;

        assert_eq!(IdentifierMask::extract_value_from_high(high), 0x0000_0001);

        // Classic bit pattern.
        let high: u32 = 0xDEAD_BEEF;

        assert_eq!(IdentifierMask::extract_value_from_high(high), 0x5EAD_BEEF);
    }

    #[test]
    fn pack_kind_bits() {
        // All bits are ones expect the most significant bit, which is zero
        let high: u32 = 0x7FFF_FFFF;

        assert_eq!(
            IdentifierMask::pack_kind_into_high(high, IdKind::Placeholder),
            0xFFFF_FFFF
        );

        // Arbitrary bit pattern
        let high: u32 = 0x00FF_FF00;

        assert_eq!(
            IdentifierMask::pack_kind_into_high(high, IdKind::Entity),
            // Remains unchanged as before
            0x00FF_FF00
        );

        // Bit pattern that almost spells a word
        let high: u32 = 0x40FF_EEEE;

        assert_eq!(
            IdentifierMask::pack_kind_into_high(high, IdKind::Placeholder),
            0xC0FF_EEEE // Milk and no sugar, please.
        );
    }

    #[test]
    fn pack_into_u64() {
        let high: u32 = 0x7FFF_FFFF;
        let low: u32 = 0x0000_00CC;

        assert_eq!(
            IdentifierMask::pack_into_u64(low, high),
            0x7FFF_FFFF_0000_00CC
        );
    }

    #[test]
    fn incrementing_masked_nonzero_high_is_safe() {
        // Adding from lowest value with lowest to highest increment
        // No result should ever be greater than 0x7FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_masked_high_by(NonZeroU32::MIN, 0)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MIN, 1)
        );
        assert_eq!(
            NonZeroU32::new(3).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MIN, 2)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_masked_high_by(NonZeroU32::MIN, HIGH_MASK)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_masked_high_by(NonZeroU32::MIN, u32::MAX)
        );
        // Adding from absolute highest value with lowest to highest increment
        // No result should ever be greater than 0x7FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MAX, 0)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_masked_high_by(NonZeroU32::MAX, 1)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MAX, 2)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MAX, HIGH_MASK)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::MAX, u32::MAX)
        );
        // Adding from actual highest value with lowest to highest increment
        // No result should ever be greater than 0x7FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::new(HIGH_MASK).unwrap(), 0)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_masked_high_by(NonZeroU32::new(HIGH_MASK).unwrap(), 1)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::new(HIGH_MASK).unwrap(), 2)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::new(HIGH_MASK).unwrap(), HIGH_MASK)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_masked_high_by(NonZeroU32::new(HIGH_MASK).unwrap(), u32::MAX)
        );
    }
}
