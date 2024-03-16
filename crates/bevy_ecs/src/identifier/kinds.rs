/// The kinds of ID that [`super::Identifier`] can represent. Each
/// variant imposes different usages of the low/high segments
/// of the ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum IdKind {
    /// An ID variant that is compatible with [`crate::entity::Entity`].
    Entity = 0,
    /// A future ID variant.
    Placeholder = 0b1000_0000,
}
