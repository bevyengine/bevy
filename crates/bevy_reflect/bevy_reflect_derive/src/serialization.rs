use crate::derive_data::{EnumVariant, StructField};
use crate::field_attributes::ReflectIgnoreBehavior;
use bit_set::BitSet;

/// A bitset of fields to ignore for serialization.
///
/// This uses the `#[reflect(skip_serializing)]` and `#[reflect(ignore)]` attributes to determine
/// which fields to mark as skippable for serialization.
///
/// This data is encoded into a bitset over the fields' indices.
///
/// For enums, this also contains the name of the variant associated with the bitset.
pub(crate) enum SerializationDenylist {
    Struct(BitSet<usize>),
    Enum(Vec<(String, BitSet<usize>)>),
}

impl SerializationDenylist {
    /// Create a new bitset for a struct's fields.
    ///
    /// This will return a [`SerializationDenylist::Struct`] value.
    pub fn from_struct_fields<'a>(fields: impl Iterator<Item = &'a StructField<'a>>) -> Self {
        Self::Struct(Self::generate_bitset(fields))
    }

    /// Create a new bitset for an enum's fields.
    ///
    /// This will return a [`SerializationDenylist::Enum`] value.
    pub fn from_enum_variants<'a>(variants: impl Iterator<Item = &'a EnumVariant<'a>>) -> Self {
        Self::Enum(
            variants
                .map(|variant| {
                    let name = variant.data.ident.to_string();
                    let denylist = Self::generate_bitset(variant.fields().iter());
                    (name, denylist)
                })
                .collect(),
        )
    }

    /// Converts an iterator over fields to a bitset of ignored members.
    ///
    /// Takes into account the fact that always ignored (non-reflected) members are skipped.
    ///
    /// # Example
    /// ```rust,ignore
    /// pub struct HelloWorld {
    ///     reflected_field: u32      // index: 0
    ///
    ///     #[reflect(ignore)]
    ///     non_reflected_field: u32  // index: N/A (not 1!)
    ///
    ///     #[reflect(skip_serializing)]
    ///     non_serialized_field: u32 // index: 1
    /// }
    /// ```
    /// Would convert to the `0b01` bitset (i.e second field is NOT serialized).
    /// Keep in mind, however, that it is always recommended that
    /// `#[reflect(skip_serializing)]` comes _before_ `#[reflect(ignore)]`.
    /// The example above is meant for demonstration purposes only.
    ///
    fn generate_bitset<'a>(fields: impl Iterator<Item = &'a StructField<'a>>) -> BitSet<usize> {
        let mut bitset = BitSet::default();

        fields.fold(0, |next_idx, member| match member.attrs.ignore {
            ReflectIgnoreBehavior::IgnoreAlways => next_idx,
            ReflectIgnoreBehavior::IgnoreSerialization => {
                bitset.insert(next_idx);
                next_idx + 1
            }
            ReflectIgnoreBehavior::None => next_idx + 1,
        });

        bitset
    }
}
