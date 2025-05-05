use core::marker::PhantomData;

use derive_more::derive::{Deref, DerefMut};

use crate::{
    bundle::Bundle,
    component::{ComponentId, Components},
    query::Access,
};

/// Access scopes define the set of [`Component`]s accessible by an entity
/// reference type.
///
/// The following scopes are available:
/// - [`Full`]: Allows reading and writing all components.
/// - [`Partial`]: Allows reading and writing only the components allowed by the
///   held [`Access<ComponentId>`].
/// - [`Except`]: Allows reading and writing all components except those
///   contained in the [`Bundle`] `B`.
/// - [`Only`]: Allows reading and writing only the components contained in the
///   [`Bundle`] `B`.
///
/// # Safety
///
/// Implementors must ensure that [`AccessScope::as_ref`] provides access to the same
/// set of components as `Self`.
///
/// [`Component`]: crate::component::Component
pub unsafe trait AccessScope {
    /// Associated type to take this scope by reference.
    ///
    /// By using an associated type rather than `&scope` directly, we can
    /// ensure that [`Full`] entity references always stay as [`Full`] and don't
    /// become `&Full`, for example.
    type AsRef<'a>: AccessScope
    where
        Self: 'a;

    /// Takes this scope by reference.
    fn as_ref(&self) -> Self::AsRef<'_>;

    /// Returns `true` if the entity has read access to the component with the
    /// given [`ComponentId`].
    fn can_read(&self, components: &Components, id: ComponentId) -> bool;

    /// Returns `true` if the entity has write access to the component with the
    /// given [`ComponentId`].
    fn can_write(&self, components: &Components, id: ComponentId) -> bool;
}

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl<S: AccessScope> AccessScope for &S {
    type AsRef<'a>
        = S::AsRef<'a>
    where
        Self: 'a;

    fn as_ref(&self) -> Self::AsRef<'_> {
        (**self).as_ref()
    }

    fn can_read(&self, components: &Components, component: ComponentId) -> bool {
        (**self).can_read(components, component)
    }

    fn can_write(&self, components: &Components, component: ComponentId) -> bool {
        (**self).can_write(components, component)
    }
}

/// An [`AccessScope`] that allows reading and writing all components.
#[derive(Clone, Copy)]
pub struct Full;

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl AccessScope for Full {
    type AsRef<'a> = Full;

    fn as_ref(&self) -> Self::AsRef<'_> {
        *self
    }

    fn can_read(&self, _: &Components, _: ComponentId) -> bool {
        true
    }

    fn can_write(&self, _: &Components, _: ComponentId) -> bool {
        true
    }
}

/// An [`AccessScope`] that allows reading and writing only the components allowed by
/// the held [`Access<ComponentId>`].
#[derive(Clone, Deref, DerefMut)]
pub struct Partial(pub Access<ComponentId>);

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl AccessScope for Partial {
    type AsRef<'a> = &'a Partial;

    fn as_ref(&self) -> Self::AsRef<'_> {
        self
    }

    fn can_read(&self, _: &Components, id: ComponentId) -> bool {
        self.0.has_component_read(id)
    }

    fn can_write(&self, _: &Components, id: ComponentId) -> bool {
        self.0.has_component_write(id)
    }
}

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl AccessScope for Access<ComponentId> {
    type AsRef<'a> = &'a Access<ComponentId>;

    fn as_ref(&self) -> Self::AsRef<'_> {
        self
    }

    fn can_read(&self, _: &Components, id: ComponentId) -> bool {
        self.has_component_read(id)
    }

    fn can_write(&self, _: &Components, id: ComponentId) -> bool {
        self.has_component_write(id)
    }
}

/// An [`AccessScope`] that allows reading and writing all components except those
/// contained in the [`Bundle`] `B`.
pub struct Except<B: Bundle>(PhantomData<B>);

impl<B: Bundle> Clone for Except<B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Bundle> Copy for Except<B> {}

impl<B: Bundle> Default for Except<B> {
    fn default() -> Self {
        Except(PhantomData)
    }
}

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl<B: Bundle> AccessScope for Except<B> {
    type AsRef<'a> = Except<B>;

    fn as_ref(&self) -> Self::AsRef<'_> {
        *self
    }

    fn can_read(&self, components: &Components, id: ComponentId) -> bool {
        let mut found = false;
        B::get_component_ids(components, &mut |maybe_id| {
            if let Some(bid) = maybe_id {
                found = found || bid == id;
            }
        });
        !found
    }

    fn can_write(&self, components: &Components, id: ComponentId) -> bool {
        self.can_read(components, id)
    }
}

/// An [`AccessScope`] that allows reading and writing only the components contained in
/// the [`Bundle`] `B`.
pub struct Only<B: Bundle>(PhantomData<B>);

impl<B: Bundle> Clone for Only<B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Bundle> Copy for Only<B> {}

impl<B: Bundle> Default for Only<B> {
    fn default() -> Self {
        Only(PhantomData)
    }
}

// SAFETY: `as_ref` refers to the same set of components as `Self`
unsafe impl<B: Bundle> AccessScope for Only<B> {
    type AsRef<'a> = Only<B>;

    fn as_ref(&self) -> Self::AsRef<'_> {
        *self
    }

    fn can_read(&self, components: &Components, id: ComponentId) -> bool {
        let mut found = false;
        B::get_component_ids(components, &mut |maybe_id| {
            if let Some(bid) = maybe_id {
                found = found || bid == id;
            }
        });
        found
    }

    fn can_write(&self, components: &Components, id: ComponentId) -> bool {
        self.can_read(components, id)
    }
}
