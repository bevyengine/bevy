//! A module for the unified [`Identifier`] ID struct, for use as a representation
//! of multiple types of IDs in a single, packed type. Allows for describing an [`crate::entity::Entity`],
//! or other IDs that can be packed and expressed within a `u64` sized type.
//! [`Identifier`]s cannot be created directly, only able to be converted from other
//! compatible IDs.
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

use self::{error::IdentifierError, kinds::IdKind, masks::IdentifierMask};
use std::{hash::Hash, num::NonZeroU32};

pub mod error;
pub(crate) mod kinds;
pub(crate) mod masks;

/// A unified identifier for all entity and similar IDs.
/// Has the same size as a `u64` integer, but the layout is split between a 32-bit low
/// segment, a 31-bit high segment, and the significant bit reserved as type flags to denote
/// entity kinds.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect_value(Debug, Hash, PartialEq))]
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
    /// Construct a new [`Identifier`]. The `high` parameter is masked with the
    /// `kind` so to pack the high value and bit flags into the same field.
    #[inline(always)]
    pub const fn new(low: u32, high: u32, kind: IdKind) -> Result<Self, IdentifierError> {
        // the high bits are masked to cut off the most significant bit
        // as these are used for the type flags. This means that the high
        // portion is only 31 bits, but this still provides 2^31
        // values/kinds/ids that can be stored in this segment.
        let masked_value = IdentifierMask::extract_value_from_high(high);

        let packed_high = IdentifierMask::pack_kind_into_high(masked_value, kind);

        // If the packed high component ends up being zero, that means that we tried
        // to initialise an Identifier into an invalid state.
        if packed_high == 0 {
            Err(IdentifierError::InvalidIdentifier)
        } else {
            // SAFETY: The high value has been checked to ensure it is never
            // zero.
            unsafe {
                Ok(Self {
                    low,
                    high: NonZeroU32::new_unchecked(packed_high),
                })
            }
        }
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
        let id = Self::try_from_bits(value);

        match id {
            Ok(id) => id,
            Err(_) => panic!("Attempted to initialise invalid bits as an id"),
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
    use super::*;

    #[test]
    fn id_construction() {
        let id = Identifier::new(12, 55, IdKind::Entity).unwrap();

        assert_eq!(id.low(), 12);
        assert_eq!(id.high().get(), 55);
        assert_eq!(
            IdentifierMask::extract_kind_from_high(id.high().get()),
            IdKind::Entity
        );
    }

    #[test]
    fn from_bits() {
        // This high value should correspond to the max high() value
        // and also Entity flag.
        let high = 0x7FFFFFFF;
        let low = 0xC;
        let bits: u64 = high << u32::BITS | low;

        let id = Identifier::try_from_bits(bits).unwrap();

        assert_eq!(id.to_bits(), 0x7FFFFFFF0000000C);
        assert_eq!(id.low(), low as u32);
        assert_eq!(id.high().get(), 0x7FFFFFFF);
        assert_eq!(
            IdentifierMask::extract_kind_from_high(id.high().get()),
            IdKind::Entity
        );
    }

    #[rustfmt::skip]
    #[test]
    #[allow(clippy::nonminimal_bool)] // This is intentionally testing `lt` and `ge` as separate functions.
    fn id_comparison() {
        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() == Identifier::new(123, 456, IdKind::Entity).unwrap());
        assert!(Identifier::new(123, 456, IdKind::Placeholder).unwrap() == Identifier::new(123, 456, IdKind::Placeholder).unwrap());
        assert!(Identifier::new(123, 789, IdKind::Entity).unwrap() != Identifier::new(123, 456, IdKind::Entity).unwrap());
        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() != Identifier::new(123, 789, IdKind::Entity).unwrap());
        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() != Identifier::new(456, 123, IdKind::Entity).unwrap());
        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() != Identifier::new(123, 456, IdKind::Placeholder).unwrap());

        // ordering is by flag then high then by low

        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() >= Identifier::new(123, 456, IdKind::Entity).unwrap());
        assert!(Identifier::new(123, 456, IdKind::Entity).unwrap() <= Identifier::new(123, 456, IdKind::Entity).unwrap());
        assert!(!(Identifier::new(123, 456, IdKind::Entity).unwrap() < Identifier::new(123, 456, IdKind::Entity).unwrap()));
        assert!(!(Identifier::new(123, 456, IdKind::Entity).unwrap() > Identifier::new(123, 456, IdKind::Entity).unwrap()));

        assert!(Identifier::new(9, 1, IdKind::Entity).unwrap() < Identifier::new(1, 9, IdKind::Entity).unwrap());
        assert!(Identifier::new(1, 9, IdKind::Entity).unwrap() > Identifier::new(9, 1, IdKind::Entity).unwrap());

        assert!(Identifier::new(9, 1, IdKind::Entity).unwrap() < Identifier::new(9, 1, IdKind::Placeholder).unwrap());
        assert!(Identifier::new(1, 9, IdKind::Placeholder).unwrap() > Identifier::new(1, 9, IdKind::Entity).unwrap());

        assert!(Identifier::new(1, 1, IdKind::Entity).unwrap() < Identifier::new(2, 1, IdKind::Entity).unwrap());
        assert!(Identifier::new(1, 1, IdKind::Entity).unwrap() <= Identifier::new(2, 1, IdKind::Entity).unwrap());
        assert!(Identifier::new(2, 2, IdKind::Entity).unwrap() > Identifier::new(1, 2, IdKind::Entity).unwrap());
        assert!(Identifier::new(2, 2, IdKind::Entity).unwrap() >= Identifier::new(1, 2, IdKind::Entity).unwrap());
    }
}
