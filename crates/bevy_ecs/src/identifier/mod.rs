//! A module for the unified [`Identifier`] ID struct, for use as a representation
//! of multiple types of IDs in a single, packed type. Allows for describing an [`crate::entity::Entity`],
//! or other IDs that can be packed and expressed within a `u64` sized type.
//! [`Identifier`]s cannot be created directly, only able to be converted from other
//! compatible IDs.
use self::{
    bits::IdentifierFlagBits, error::IdentifierError, kinds::IdKind, masks::IdentifierMask,
};
use std::{hash::Hash, num::NonZeroU32};

pub mod bits;
pub mod error;
pub mod kinds;
pub(crate) mod masks;

/// A unified identifier for all entity and similar IDs.
/// Has the same size as a `u64` integer, but the layout is split between a 32-bit low
/// segment, a 30-bit high segment, and the 2 most significant bits reserved as bit flags.
#[derive(Debug, Clone, Copy)]
// Alignment repr necessary to allow LLVM to better output
// optimised codegen for `to_bits`, `PartialEq` and `Ord`.
#[repr(C, align(8))]
pub struct Identifier {
    // Do not reorder the fields here. The ordering is explicitly used by repr(C)
    // to make this struct equivalent to a u64.
    #[cfg(target_endian = "little")]
    low: u32,
    high: NonZeroU32,
    #[cfg(target_endian = "big")]
    low: u32,
}

impl Identifier {
    /// Construct a new [`Identifier`]. The `high` parameter is masked so to pack
    /// the high value and bit flags into the same field.
    #[inline(always)]
    pub const fn new(
        low: u32,
        high: u32,
        flags: IdentifierFlagBits,
    ) -> Result<Identifier, IdentifierError> {
        // the high bits are masked to cut off the most significant bit
        // as these are used for the type flags. This means that the high
        // portion is only 30 bits, but this still provides 2^30
        // values/kinds/ids that can be stored in this segment.
        let masked_value = IdentifierMask::extract_value_from_high(high);

        let packed_high = IdentifierMask::pack_flags_into_high(masked_value, flags);

        // If the packed high component ends up being zero, that means that we tried
        // to initialise an Identifier into an invalid state.
        if packed_high == 0 {
            Err(IdentifierError::InvalidIdentifier)
        } else {
            // SAFETY: The high value has been checked to ensure it is never
            // zero.
            unsafe {
                Ok(Self::from_parts(
                    low,
                    NonZeroU32::new_unchecked(packed_high),
                ))
            }
        }
    }

    #[inline(always)]
    #[must_use]
    pub(crate) const fn from_parts(low: u32, high: NonZeroU32) -> Identifier {
        Self { low, high }
    }

    /// Returns the value of the low segment of the [`Identifier`].
    #[inline(always)]
    pub const fn low(self) -> u32 {
        self.low
    }

    /// Returns the value of the high segment of the [`Identifier`]. This
    /// does not apply any masking.
    #[inline(always)]
    pub const fn high(self) -> NonZeroU32 {
        self.high
    }

    /// Returns the masked value of the high segment of the [`Identifier`].
    /// Does not include the flag bits.
    #[inline(always)]
    pub const fn masked_high(self) -> u32 {
        IdentifierMask::extract_value_from_high(self.high.get())
    }

    /// Returns the kind of [`Identifier`] from the high segment.
    #[inline(always)]
    pub const fn kind(self) -> IdKind {
        IdentifierMask::extract_kind_from_high(self.high.get())
    }

    /// Returns with the [`Identifier`] is in a `hidden` state.
    #[inline(always)]
    pub const fn is_togglable(self) -> bool {
        self.flags().contains(IdentifierFlagBits::IS_TOGGLABLE)
    }

    /// Returns whether the [`Identifier`] is a [`IdKind::Placeholder`]
    /// kind or not.
    #[inline(always)]
    pub const fn is_placeholder(self) -> bool {
        self.flags().contains(IdentifierFlagBits::IS_PLACEHOLDER)
    }

    /// Returns the flag bits stored in the [`Identifier`].
    #[inline(always)]
    pub const fn flags(self) -> IdentifierFlagBits {
        IdentifierMask::extract_flags_from_high(self.high.get())
    }

    /// Returns a `hidden` [`Identifier`].
    #[inline(always)]
    #[must_use]
    pub const fn set_hidden(self, state: bool) -> Identifier {
        Self {
            low: self.low,
            // SAFETY: the high component will always be non-zero due to either the
            // placeholder flag or the value component not being modified and either
            // one guaranteed to be one due to Identifier being initialised with the
            // correct invariants checked. As such, modifying the hidden bit will
            // never result in a zero value.
            high: unsafe {
                NonZeroU32::new_unchecked(IdentifierMask::set_togglable_flag_in_high(
                    self.high.get(),
                    state,
                ))
            },
        }
    }

    /// Convert the [`Identifier`] into a `u64`.
    #[inline(always)]
    pub const fn to_bits(self) -> u64 {
        IdentifierMask::pack_into_u64(self.low, self.high.get())
    }

    /// Convert a `u64` into an [`Identifier`].
    ///
    /// # Panics
    ///
    /// This method will likely panic if given `u64` values that did not come from [`Identifier::to_bits`].
    #[inline(always)]
    pub const fn from_bits(value: u64) -> Self {
        #[inline(never)]
        #[cold]
        #[track_caller]
        const fn invalid_id() -> ! {
            panic!("Attempted to initialise invalid bits as an id");
        }

        let id = Self::try_from_bits(value);

        match id {
            Ok(id) => id,
            Err(_) => invalid_id(),
        }
    }

    /// Convert a `u64` into an [`Identifier`].
    ///
    /// This method is the fallible counterpart to [`Identifier::from_bits`].
    #[inline(always)]
    pub const fn try_from_bits(value: u64) -> Result<Self, IdentifierError> {
        let high = NonZeroU32::new(IdentifierMask::get_high(value));

        match high {
            Some(high) => Ok(Self {
                low: IdentifierMask::get_low(value),
                high,
            }),
            None => Err(IdentifierError::InvalidIdentifier),
        }
    }
}

// By not short-circuiting in comparisons, we get better codegen.
// See <https://github.com/rust-lang/rust/issues/117800>
impl PartialEq for Identifier {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // By using `to_bits`, the codegen can be optimised out even
        // further potentially. Relies on the correct alignment/field
        // order of `Entity`.
        self.to_bits() == other.to_bits()
    }
}

impl Eq for Identifier {}

// The derive macro codegen output is not optimal and can't be optimised as well
// by the compiler. This impl resolves the issue of non-optimal codegen by relying
// on comparing against the bit representation of `Entity` instead of comparing
// the fields. The result is then LLVM is able to optimise the codegen for Entity
// far beyond what the derive macro can.
// See <https://github.com/rust-lang/rust/issues/106107>
impl PartialOrd for Identifier {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Make use of our `Ord` impl to ensure optimal codegen output
        Some(self.cmp(other))
    }
}

// The derive macro codegen output is not optimal and can't be optimised as well
// by the compiler. This impl resolves the issue of non-optimal codegen by relying
// on comparing against the bit representation of `Entity` instead of comparing
// the fields. The result is then LLVM is able to optimise the codegen for Entity
// far beyond what the derive macro can.
// See <https://github.com/rust-lang/rust/issues/106107>
impl Ord for Identifier {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // This will result in better codegen for ordering comparisons, plus
        // avoids pitfalls with regards to macro codegen relying on property
        // position when we want to compare against the bit representation.
        self.to_bits().cmp(&other.to_bits())
    }
}

impl Hash for Identifier {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_bits().hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::identifier::masks::HIGH_MASK;

    use super::*;

    #[test]
    fn id_construction() {
        let id = Identifier::new(12, 55, IdentifierFlagBits::empty()).unwrap();

        assert_eq!(id.low(), 12);
        assert_eq!(id.high().get(), 55);
        assert_eq!(id.flags(), IdentifierFlagBits::empty());
        assert!(!id.is_togglable());
    }

    #[test]
    fn from_bits() {
        // This high value should correspond to the max high() value
        // and also Entity flag.
        let high = 0x7FFF_FFFF;
        let low = 0xC;
        let bits: u64 = high << u32::BITS | low;

        let id = Identifier::try_from_bits(bits).unwrap();

        assert_eq!(id.to_bits(), 0x7FFF_FFFF_0000_000C);
        assert_eq!(id.low(), low as u32);
        assert_eq!(id.masked_high(), HIGH_MASK);
        assert!(id.flags().contains(IdentifierFlagBits::IS_TOGGLABLE));
        assert!(!id.flags().contains(IdentifierFlagBits::IS_PLACEHOLDER));
        assert!(id.is_togglable());
        assert!(!id.is_placeholder());
    }

    #[test]
    fn hidden_states() {
        let high = 0x7FFF_FFFF;
        let low = 0xC;
        let bits: u64 = high << u32::BITS | low;

        let id = Identifier::try_from_bits(bits).unwrap();

        assert!(id.is_togglable());

        let id = id.set_hidden(false);

        assert_eq!(id.to_bits(), 0x3FFF_FFFF_0000_000C);
        assert!(!id.is_togglable());
    }

    #[test]
    fn id_comparison() {
        // This is intentionally testing `lt` and `ge` as separate functions.
        #![allow(clippy::nonminimal_bool)]

        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                == Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::IS_PLACEHOLDER).unwrap()
                == Identifier::new(123, 456, IdentifierFlagBits::IS_PLACEHOLDER).unwrap()
        );
        assert!(
            Identifier::new(123, 789, IdentifierFlagBits::all()).unwrap()
                != Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                != Identifier::new(123, 789, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                != Identifier::new(456, 123, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                != Identifier::new(123, 456, IdentifierFlagBits::IS_PLACEHOLDER).unwrap()
        );

        // ordering is by flag then high then by low

        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                >= Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                <= Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            !(Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                < Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap())
        );
        assert!(
            !(Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap()
                > Identifier::new(123, 456, IdentifierFlagBits::empty()).unwrap())
        );

        assert!(
            Identifier::new(9, 1, IdentifierFlagBits::empty()).unwrap()
                < Identifier::new(1, 9, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(1, 9, IdentifierFlagBits::empty()).unwrap()
                > Identifier::new(9, 1, IdentifierFlagBits::empty()).unwrap()
        );

        assert!(
            Identifier::new(9, 1, IdentifierFlagBits::empty()).unwrap()
                < Identifier::new(9, 1, IdentifierFlagBits::IS_PLACEHOLDER).unwrap()
        );
        assert!(
            Identifier::new(1, 9, IdentifierFlagBits::IS_PLACEHOLDER).unwrap()
                > Identifier::new(1, 9, IdentifierFlagBits::empty()).unwrap()
        );

        assert!(
            Identifier::new(1, 1, IdentifierFlagBits::empty()).unwrap()
                < Identifier::new(2, 1, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(1, 1, IdentifierFlagBits::empty()).unwrap()
                <= Identifier::new(2, 1, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(2, 2, IdentifierFlagBits::empty()).unwrap()
                > Identifier::new(1, 2, IdentifierFlagBits::empty()).unwrap()
        );
        assert!(
            Identifier::new(2, 2, IdentifierFlagBits::empty()).unwrap()
                >= Identifier::new(1, 2, IdentifierFlagBits::empty()).unwrap()
        );
    }
}
