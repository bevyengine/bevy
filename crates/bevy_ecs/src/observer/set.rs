use crate::{define_label, intern::Interned};

pub use bevy_ecs_macros::ObserverSet;

define_label!(
    /// Observer sets are tag-like labels that can be used to group observers together.
    ///
    /// They are intended to support observer ordering and shared observer configuration.
    /// Observer set configuration is not yet wired into dispatch; this trait is the
    /// foundation used by the observer storage and builder APIs.
    ///
    /// # Defining new observer sets
    ///
    /// To create a new observer set, use the `#[derive(ObserverSet)]` macro.
    ///
    /// ```
    /// use bevy_ecs::observer::ObserverSet;
    ///
    /// #[derive(ObserverSet, Debug, Clone, PartialEq, Eq, Hash)]
    /// struct GameplayObservers;
    /// ```
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(ObserverSet)]`"
    )]
    ObserverSet,
    OBSERVER_SET_INTERNER
);

/// A shorthand for `Interned<dyn ObserverSet>`.
pub type InternedObserverSet = Interned<dyn ObserverSet>;

#[cfg(test)]
mod tests {
    use super::ObserverSet;

    #[test]
    fn test_derive_observer_set() {
        #[derive(ObserverSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        struct Smoke;

        assert_eq!(Smoke.intern(), Smoke.intern());
    }
}
