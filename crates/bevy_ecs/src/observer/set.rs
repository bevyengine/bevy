use alloc::{vec, vec::Vec};
use variadics_please::all_tuples;

use crate::{
    define_label,
    entity::Entity,
    intern::Interned,
    observer::{EdgeTarget, IntoObserver, Observer},
};

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

/// A target that can be used in observer ordering constraints.
#[doc(hidden)]
pub struct ObserverOrderingTarget(EdgeTarget);

impl ObserverOrderingTarget {
    pub(crate) fn into_edge_target(self) -> EdgeTarget {
        self.0
    }
}

/// Types that can be used as an observer ordering target.
pub trait IntoObserverOrderingTarget {
    /// Convert this value into an observer ordering target.
    fn into_observer_ordering_target(self) -> ObserverOrderingTarget;
}

impl IntoObserverOrderingTarget for Entity {
    fn into_observer_ordering_target(self) -> ObserverOrderingTarget {
        ObserverOrderingTarget(EdgeTarget::Entity(self))
    }
}

impl<S: ObserverSet> IntoObserverOrderingTarget for S {
    fn into_observer_ordering_target(self) -> ObserverOrderingTarget {
        ObserverOrderingTarget(EdgeTarget::Set(self.intern()))
    }
}

/// A configured collection of observers.
pub struct ObserverConfigs {
    pub(crate) observers: Vec<Observer>,
    pub(crate) chain: bool,
}

impl ObserverConfigs {
    fn new(observer: Observer) -> Self {
        Self {
            observers: vec![observer],
            chain: false,
        }
    }

    /// Add every observer in this collection to `set`.
    pub fn in_set<S: ObserverSet>(mut self, set: S) -> Self {
        let set = set.intern();
        for observer in &mut self.observers {
            observer.descriptor.sets.push(set);
        }
        self
    }

    /// Run every observer in this collection before `target`.
    pub fn before(mut self, target: impl IntoObserverOrderingTarget) -> Self {
        let target = target.into_observer_ordering_target().into_edge_target();
        for observer in &mut self.observers {
            observer.before_inner(target.clone());
        }
        self
    }

    /// Run every observer in this collection after `target`.
    pub fn after(mut self, target: impl IntoObserverOrderingTarget) -> Self {
        let target = target.into_observer_ordering_target().into_edge_target();
        for observer in &mut self.observers {
            observer.after_inner(target.clone());
        }
        self
    }

    /// Treat this observer collection as a sequence.
    pub fn chain(mut self) -> Self {
        self.chain = true;
        self
    }
}

/// Types that can be converted into observer configurations.
pub trait IntoObserverConfigs<Marker>: Sized {
    /// Convert into observer configurations.
    fn into_configs(self) -> ObserverConfigs;

    /// Add these observers to `set`.
    fn in_set<S: ObserverSet>(self, set: S) -> ObserverConfigs {
        self.into_configs().in_set(set)
    }

    /// Run these observers before `target`.
    fn before<T: IntoObserverOrderingTarget>(self, target: T) -> ObserverConfigs {
        self.into_configs().before(target)
    }

    /// Run these observers after `target`.
    fn after<T: IntoObserverOrderingTarget>(self, target: T) -> ObserverConfigs {
        self.into_configs().after(target)
    }

    /// Treat this observer collection as a sequence.
    fn chain(self) -> ObserverConfigs {
        self.into_configs().chain()
    }
}

impl IntoObserverConfigs<()> for ObserverConfigs {
    fn into_configs(self) -> ObserverConfigs {
        self
    }
}

impl<I, Marker> IntoObserverConfigs<Marker> for I
where
    I: IntoObserver<Marker>,
{
    fn into_configs(self) -> ObserverConfigs {
        ObserverConfigs::new(self.into_observer())
    }
}

#[doc(hidden)]
pub struct ObserverConfigTupleMarker;

macro_rules! impl_observer_config_tuple {
    ($(#[$meta:meta])* $(($param: ident, $observer: ident)),*) => {
        $(#[$meta])*
        impl<$($param, $observer),*> IntoObserverConfigs<(ObserverConfigTupleMarker, $($param,)*)> for ($($observer,)*)
        where
            $($observer: IntoObserverConfigs<$param>),*
        {
            #[expect(
                clippy::allow_attributes,
                reason = "We are inside a macro, and as such, `non_snake_case` is not guaranteed to apply."
            )]
            #[allow(
                non_snake_case,
                reason = "Variable names are provided by the macro caller, not by us."
            )]
            fn into_configs(self) -> ObserverConfigs {
                let ($($observer,)*) = self;
                let mut observers = Vec::new();
                $(
                    observers.extend($observer.into_configs().observers);
                )*
                ObserverConfigs {
                    observers,
                    chain: false,
                }
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_observer_config_tuple,
    1,
    20,
    P,
    S
);

/// A configured collection of observer sets.
#[derive(Clone, Default)]
pub struct ObserverSetConfigs {
    pub(crate) sets: Vec<InternedObserverSet>,
    pub(crate) hierarchy: Vec<(InternedObserverSet, InternedObserverSet)>,
    pub(crate) edges: Vec<(InternedObserverSet, InternedObserverSet)>,
    pub(crate) chain: bool,
}

impl ObserverSetConfigs {
    fn new(set: InternedObserverSet) -> Self {
        Self {
            sets: vec![set],
            hierarchy: Vec::new(),
            edges: Vec::new(),
            chain: false,
        }
    }

    /// Add every observer set in this collection to `parent`.
    pub fn in_set<S: ObserverSet>(mut self, parent: S) -> Self {
        let parent = parent.intern();
        for &set in &self.sets {
            self.hierarchy.push((set, parent));
        }
        self
    }

    /// Run every observer in these sets before every observer in `target`.
    pub fn before<S: ObserverSet>(mut self, target: S) -> Self {
        let target = target.intern();
        for &set in &self.sets {
            self.edges.push((set, target));
        }
        self
    }

    /// Run every observer in these sets after every observer in `target`.
    pub fn after<S: ObserverSet>(mut self, target: S) -> Self {
        let target = target.intern();
        for &set in &self.sets {
            self.edges.push((target, set));
        }
        self
    }

    /// Treat this observer set collection as a sequence.
    pub fn chain(mut self) -> Self {
        self.chain = true;
        self
    }

    pub(crate) fn add_chain_edges(&mut self) {
        if !self.chain {
            return;
        }

        for sets in self.sets.windows(2) {
            let edge = (sets[0], sets[1]);
            if !self.edges.contains(&edge) {
                self.edges.push(edge);
            }
        }
    }
}

/// Types that can be converted into observer set configurations.
pub trait IntoObserverSetConfigs<Marker>: Sized {
    /// Convert into observer set configurations.
    fn into_configs(self) -> ObserverSetConfigs;

    /// Add these observer sets to `parent`.
    fn in_set<S: ObserverSet>(self, parent: S) -> ObserverSetConfigs {
        self.into_configs().in_set(parent)
    }

    /// Run these observer sets before `target`.
    fn before<S: ObserverSet>(self, target: S) -> ObserverSetConfigs {
        self.into_configs().before(target)
    }

    /// Run these observer sets after `target`.
    fn after<S: ObserverSet>(self, target: S) -> ObserverSetConfigs {
        self.into_configs().after(target)
    }

    /// Treat this observer set collection as a sequence.
    fn chain(self) -> ObserverSetConfigs {
        self.into_configs().chain()
    }
}

impl IntoObserverSetConfigs<()> for ObserverSetConfigs {
    fn into_configs(self) -> ObserverSetConfigs {
        self
    }
}

impl<S: ObserverSet> IntoObserverSetConfigs<()> for S {
    fn into_configs(self) -> ObserverSetConfigs {
        ObserverSetConfigs::new(self.intern())
    }
}

#[doc(hidden)]
pub struct ObserverSetConfigTupleMarker;

macro_rules! impl_observer_set_config_tuple {
    ($(#[$meta:meta])* $(($param: ident, $set: ident)),*) => {
        $(#[$meta])*
        impl<$($param, $set),*> IntoObserverSetConfigs<(ObserverSetConfigTupleMarker, $($param,)*)> for ($($set,)*)
        where
            $($set: IntoObserverSetConfigs<$param>),*
        {
            #[expect(
                clippy::allow_attributes,
                reason = "We are inside a macro, and as such, `non_snake_case` is not guaranteed to apply."
            )]
            #[allow(
                non_snake_case,
                reason = "Variable names are provided by the macro caller, not by us."
            )]
            fn into_configs(self) -> ObserverSetConfigs {
                let ($($set,)*) = self;
                let mut configs = ObserverSetConfigs::default();
                $(
                    let config = $set.into_configs();
                    configs.sets.extend(config.sets);
                    configs.hierarchy.extend(config.hierarchy);
                    configs.edges.extend(config.edges);
                )*
                configs
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_observer_set_config_tuple,
    1,
    20,
    P,
    S
);

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
