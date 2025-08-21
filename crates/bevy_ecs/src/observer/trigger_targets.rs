//! Stores the [`TriggerTargets`] trait.

use crate::{entity::Entity, event::EntityComponents};
use alloc::vec::Vec;

/// Represents a collection of targets for a specific [`On`] instance of an [`Event`].
///
/// When an event is triggered with [`TriggerTargets`], any [`Observer`] that watches for that specific
/// event-target combination will run.
///
/// This trait is implemented for both [`Entity`] and [`ComponentId`], allowing you to target specific entities or components.
/// It is also implemented for various collections of these types, such as [`Vec`], arrays, and tuples,
/// allowing you to trigger events for multiple targets at once.
pub trait EventTargets<T>: Send + Sync {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a;
}

impl<L: EventTargets<T> + ?Sized, T: 'static> EventTargets<T> for &L {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a,
    {
        (**self).targets()
    }
}

impl EventTargets<Entity> for Entity {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a Entity> + 'a
    where
        Entity: 'a,
    {
        core::iter::once(self)
    }
}

impl<'a> EventTargets<EntityComponents<'a>> for EntityComponents<'a> {
    fn targets<'b>(&'b self) -> impl Iterator<Item = &'b EntityComponents<'a>> + 'b
    where
        EntityComponents<'a>: 'b,
    {
        core::iter::once(self)
    }
}

impl EventTargets<()> for () {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a ()> + 'a
    where
        (): 'a,
    {
        core::iter::once(&())
    }
}

impl<T: Send + Sync> EventTargets<T> for Vec<T> {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a,
    {
        self.iter()
    }
}

impl<const N: usize, T: Send + Sync> EventTargets<T> for [T; N] {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a,
    {
        self.iter()
    }
}

impl<T: Send + Sync> EventTargets<T> for [T] {
    fn targets<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a
    where
        T: 'a,
    {
        self.iter()
    }
}
