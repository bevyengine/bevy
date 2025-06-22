//! Stores the [`TriggerTargets`] trait.

use crate::{component::ComponentId, prelude::*};
use alloc::vec::Vec;
use variadics_please::all_tuples;

/// Represents a collection of targets for a specific [`On`] instance of an [`Event`].
///
/// When an event is triggered with [`TriggerTargets`], any [`Observer`] that watches for that specific
/// event-target combination will run.
///
/// This trait is implemented for both [`Entity`] and [`ComponentId`], allowing you to target specific entities or components.
/// It is also implemented for various collections of these types, such as [`Vec`], arrays, and tuples,
/// allowing you to trigger events for multiple targets at once.
pub trait TriggerTargets {
    /// The components the trigger should target.
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_;

    /// The entities the trigger should target.
    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_;
}

impl<T: TriggerTargets + ?Sized> TriggerTargets for &T {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        (**self).components()
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        (**self).entities()
    }
}

impl TriggerTargets for Entity {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        [].into_iter()
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        core::iter::once(*self)
    }
}

impl TriggerTargets for ComponentId {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        core::iter::once(*self)
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        [].into_iter()
    }
}

impl<T: TriggerTargets> TriggerTargets for Vec<T> {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.iter().flat_map(T::components)
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        self.iter().flat_map(T::entities)
    }
}

impl<const N: usize, T: TriggerTargets> TriggerTargets for [T; N] {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.iter().flat_map(T::components)
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        self.iter().flat_map(T::entities)
    }
}

impl<T: TriggerTargets> TriggerTargets for [T] {
    fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
        self.iter().flat_map(T::components)
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
        self.iter().flat_map(T::entities)
    }
}

macro_rules! impl_trigger_targets_tuples {
    ($(#[$meta:meta])* $($trigger_targets: ident),*) => {
        #[expect(clippy::allow_attributes, reason = "can't guarantee violation of non_snake_case")]
        #[allow(non_snake_case, reason = "`all_tuples!()` generates non-snake-case variable names.")]
        $(#[$meta])*
        impl<$($trigger_targets: TriggerTargets),*> TriggerTargets for ($($trigger_targets,)*)
        {
            fn components(&self) -> impl Iterator<Item = ComponentId> + Clone + '_ {
                let iter = [].into_iter();
                let ($($trigger_targets,)*) = self;
                $(
                    let iter = iter.chain($trigger_targets.components());
                )*
                iter
            }

            fn entities(&self) -> impl Iterator<Item = Entity> + Clone + '_ {
                let iter = [].into_iter();
                let ($($trigger_targets,)*) = self;
                $(
                    let iter = iter.chain($trigger_targets.entities());
                )*
                iter
            }
        }
    }
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_trigger_targets_tuples,
    0,
    15,
    T
);
