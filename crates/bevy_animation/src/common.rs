//! Shared Animation types for the game engine Bevy

use bevy_core::Name;

/// Path to an entity, with [`Name`]s.
///
/// Each entity in a path must have a name.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}
