//! Provides a platform and library agnostic identifier for input sources, [`InputSource`].

use bevy_ecs::system::Resource;
use bevy_reflect::Reflect;
use std::hash::Hash;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Represents a source of input [`events`](`bevy_ecs::event::Event`).
/// This can be used to differentiate events of the same type but from different sources.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct InputSource(usize);

/// Provides unique [`InputSource`]'s when requested.
#[derive(Resource, Default, Reflect)]
pub struct InputSources {
    counter: usize,
}

impl InputSources {
    /// Attempt to register a new [`InputSource`]. Returns [`None`] if a registration is not possible.
    pub fn try_register(&mut self) -> Option<InputSource> {
        let source = InputSource(self.counter);
        self.counter = self.counter.checked_add(1)?;
        Some(source)
    }

    /// Register a new [`InputSource`].
    ///
    /// # Panics
    ///
    /// Will panic if registration fails.
    pub fn register(&mut self) -> InputSource {
        let Some(source) = self.try_register() else {
            panic!("Ran out of InputSource IDs!")
        };
        source
    }
}
