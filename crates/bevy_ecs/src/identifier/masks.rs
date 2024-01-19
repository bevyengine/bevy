use std::num::NonZeroU32;

use super::{bits::IdentifierFlagBits, kinds::IdKind};

/// Mask for extracting the value portion of a 32-bit high segment. This
/// yields 31-bits of total value, as the final bit (the most significant)
/// is reserved as a flag bit. Can be negated to extract the flag bit.
pub(crate) const HIGH_MASK: u32 = 0x3FFF_FFFF;

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

    /// Packs flag bits into the high value. This will clear any existing
    /// flags in the high component in order to be able to toggle all bits
    /// correctly.
    #[inline(always)]
    pub(crate) const fn pack_flags_into_high(value: u32, flags: IdentifierFlagBits) -> u32 {
        (value & HIGH_MASK) | flags.bits()
    }

    /// Extract the value component from a high segment of an [`super::Identifier`].
    #[inline(always)]
    pub(crate) const fn extract_value_from_high(value: u32) -> u32 {
        value & HIGH_MASK
    }

    #[inline(always)]
    pub(crate) const fn extract_flags_from_high(value: u32) -> IdentifierFlagBits {
        IdentifierFlagBits::from_bits_retain(value & !HIGH_MASK)
    }

    /// Extract the ID kind component from a high segment of an [`super::Identifier`].
    #[inline(always)]
    pub(crate) const fn extract_kind_from_high(value: u32) -> IdKind {
        let flags = Self::extract_flags_from_high(value);

        if flags.contains(IdentifierFlagBits::IS_PLACEHOLDER) {
            IdKind::Placeholder
        } else {
            IdKind::Entity
        }
    }

    #[inline(always)]
    pub(crate) const fn set_togglable_flag_in_high(value: u32, flag_state: bool) -> u32 {
        let bits = IdentifierFlagBits::from_bits_retain(value);

        if flag_state {
            bits.union(IdentifierFlagBits::IS_TOGGLABLE).bits()
        } else {
            bits.difference(IdentifierFlagBits::IS_TOGGLABLE).bits()
        }
    }

    /// Offsets a masked generation value by the specified amount, wrapping to 1 instead of 0.
    /// Will never be greater than [`HIGH_MASK`] or less than `1`, and increments will never be
    /// greater than [`HIGH_MASK`], as they are masked for safety reasons.
    ///
    /// **NOTE**: This method should only be used for `Entity`. Incrementing the high/generation
    /// value will clear any invalid flags as the generation is always incremented in the case
    /// of despawned entities that are not allocated as any particular type/kind. Only after they
    /// are spawned will they then have any flags toggled.
    #[inline(always)]
    pub(crate) const fn inc_entity_generation_by(high: NonZeroU32, increment: u32) -> NonZeroU32 {
        // Increment the masked value portion of the bits
        let lo = (high.get() & HIGH_MASK).wrapping_add(increment & HIGH_MASK);
        // Check high bits for whether we have overflowed 30 bits.
        let overflowed = ((lo & !HIGH_MASK) != 0) as u32;

        // Apply the overflow bit to the incremented value to ensure we will always
        // get a non-zero overflow. Mask to then remove unwanted high bits for the
        // final value.
        // SAFETY:
        // - The rhs is masked to never be a value greater than the mask, allowing
        //   the overflow to be tracked/accounted for.
        // - Adding the overflow flag will offet overflows to start at 1 instead of 0
        // - The sum of 0x3FFF_FFFF + 1 (overflow) == 1
        // - The sum of Ox3FFF_FFFF + 0x3FFF_FFFF == 0x3FFF_FFFF
        // - If the operation doesn't overflow at 30 bits, no offsetting takes place
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
        assert_eq!(IdentifierMask::extract_value_from_high(high), HIGH_MASK);

        // Start bit and end bit are ones.
        let high: u32 = 0xC000_0001;

        assert_eq!(IdentifierMask::extract_value_from_high(high), 0x0000_0001);

        // Classic bit pattern.
        let high: u32 = 0xDEAD_BEEF;

        assert_eq!(IdentifierMask::extract_value_from_high(high), 0x1EAD_BEEF);
    }

    #[test]
    fn pack_flag_bits() {
        // All bits are ones expect the 2 most significant bits, which are zero
        let high: u32 = 0x7FFF_FFFF;

        assert_eq!(
            IdentifierMask::pack_flags_into_high(high, IdentifierFlagBits::IS_PLACEHOLDER),
            // The IS_HIDDEN flag is cleared and the IS_PLACEHOLDER flag is enabled
            0xBFFF_FFFF
        );

        // Arbitrary bit pattern
        let high: u32 = 0x00FF_FF00;

        assert_eq!(
            IdentifierMask::pack_flags_into_high(high, IdentifierFlagBits::empty()),
            // Remains unchanged as before
            0x00FF_FF00
        );

        // Bit pattern that almost spells a word
        let high: u32 = 0x00FF_EEEE;
        let flags_to_set = IdentifierFlagBits::IS_PLACEHOLDER | IdentifierFlagBits::IS_TOGGLABLE;

        assert_eq!(
            IdentifierMask::pack_flags_into_high(high, flags_to_set),
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
    fn incrementing_entity_generation_is_safe() {
        // Adding from lowest value with lowest to highest increment
        // No result should ever be greater than 0x3FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, 0)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, 1)
        );
        assert_eq!(
            NonZeroU32::new(3).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, 2)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, HIGH_MASK)
        );
        // Adding from absolute highest value with lowest to highest increment
        // No result should ever be greater than 0x3FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MAX, 0)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MAX, 1)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MAX, 2)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MAX, HIGH_MASK)
        );
        // Adding from actual highest value with lowest to highest increment
        // No result should ever be greater than 0x3FFF_FFFF or HIGH_MASK
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::new(HIGH_MASK).unwrap(), 0)
        );
        assert_eq!(
            NonZeroU32::MIN,
            IdentifierMask::inc_entity_generation_by(NonZeroU32::new(HIGH_MASK).unwrap(), 1)
        );
        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            IdentifierMask::inc_entity_generation_by(NonZeroU32::new(HIGH_MASK).unwrap(), 2)
        );
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            IdentifierMask::inc_entity_generation_by(
                NonZeroU32::new(HIGH_MASK).unwrap(),
                HIGH_MASK
            )
        );
    }

    #[test]
    fn incrementing_generation_by_more_than_mask_size() {
        assert_eq!(
            NonZeroU32::new(HIGH_MASK).unwrap(),
            // This should be equivalent to adding by HIGH_MASK
            IdentifierMask::inc_entity_generation_by(NonZeroU32::new(HIGH_MASK).unwrap(), u32::MAX)
        );

        assert_eq!(
            NonZeroU32::MIN,
            // This should be equivalent to adding by HIGH_MASK
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, u32::MAX)
        );

        assert_eq!(
            NonZeroU32::MIN,
            // This should be equivalent to adding by 0
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, 0xC000_0000)
        );

        assert_eq!(
            NonZeroU32::new(2).unwrap(),
            // This should be equivalent to adding by 1
            IdentifierMask::inc_entity_generation_by(NonZeroU32::MIN, 0xC000_0001)
        );
    }
}
