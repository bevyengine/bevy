//! Module for defining the flag bits present in the [`super::Identifier`] format.

use std::fmt::Debug;

use bitflags::bitflags;

bitflags! {
    /// Flag bits defined for [`super::Identifier`].
    pub struct IdentifierFlagBits: u32 {
        /// Flag for determining whether an [`super::Identifier`] is a
        /// [`super::IdKind::Placeholder`] or [`super::IdKind::Entity`].
        const IS_PLACEHOLDER = 0b1000_0000_0000_0000_0000_0000_0000_0000;
        /// Flag for determining whether the [`super::Identifier`] is in a
        /// `togglable` state or not.
        const IS_TOGGLABLE = 0b0100_0000_0000_0000_0000_0000_0000_0000;

        const _ = !0;
    }
}

impl PartialEq for IdentifierFlagBits {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Debug for IdentifierFlagBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("IdentifierFlagBits").field(&self.0).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_bits_are_correctly_determined() {
        // Both flag bits are set
        let bits: u32 = 0xC0FF_EEEE;

        let flags = IdentifierFlagBits::from_bits_retain(bits);

        assert!(flags.contains(IdentifierFlagBits::IS_PLACEHOLDER));
        assert!(flags.contains(IdentifierFlagBits::IS_TOGGLABLE));

        // Only IS_PLACEHOLDER flag bit set
        let bits: u32 = 0x80FF_F00F;

        let flags = IdentifierFlagBits::from_bits_retain(bits);

        assert!(flags.contains(IdentifierFlagBits::IS_PLACEHOLDER));
        assert!(!flags.contains(IdentifierFlagBits::IS_TOGGLABLE));

        // Only IS_TOGGLABLE flag bit set
        let bits: u32 = 0x40FF_F00F;

        let flags = IdentifierFlagBits::from_bits_retain(bits);

        assert!(!flags.contains(IdentifierFlagBits::IS_PLACEHOLDER));
        assert!(flags.contains(IdentifierFlagBits::IS_TOGGLABLE));
    }
}
