use core::{marker::PhantomData, ops::Deref};

use crate::{
    bundle::Bundle,
    component::{ComponentId, Components},
    query::Access,
};

/// Defines the set of [`Component`]s accessible by the entity reference types
/// [`EntityRef`] and [`EntityMut`].
///
/// The following scopes are provided:
/// - [`All`]: Provides access to all components. This is the default scope.
/// - [`Filtered`]: Provides access only to the components specified in an
///   [`Access`]. This is used by [`FilteredEntityRef`] and [`FilteredEntityMut`].
/// - [`Except`]: Provides access to all components except those in a specified
///   [`Bundle`]. This is used by [`EntityRefExcept`] and [`EntityMutExcept`].
///
/// # Safety
///
/// Implementors must ensure that [`AccessScope::reborrow`] does not extend the
/// permissions of the scope with access it did not previously have.
///
/// [`Component`]: crate::component::Component
/// [`EntityRef`]: crate::world::EntityRef
/// [`EntityMut`]: crate::world::EntityMut
/// [`FilteredEntityRef`]: crate::world::FilteredEntityRef
/// [`FilteredEntityMut`]: crate::world::FilteredEntityMut
/// [`EntityRefExcept`]: crate::world::EntityRefExcept
/// [`EntityMutExcept`]: crate::world::EntityMutExcept
pub unsafe trait AccessScope {
    /// The reborrowed version of this scope. This is typically `Self`, but with
    /// shorter lifetimes.
    type Borrow<'a>: AccessScope
    where
        Self: 'a;

    /// Reborrows the scope for shorter lifetimes.
    fn reborrow(&self) -> Self::Borrow<'_>;

    /// Returns `true` if the scope allows reading the specified component.
    fn can_read(&self, id: ComponentId, components: &Components) -> bool;

    /// Returns `true` if the scope allows writing the specified component.
    fn can_write(&self, id: ComponentId, components: &Components) -> bool;
}

/// [`AccessScope`] that provides access to all of an entity's components. This
/// is the default scope of [`EntityRef`] and [`EntityMut`].
///
/// [`EntityRef`]: crate::world::EntityRef
/// [`EntityMut`]: crate::world::EntityMut
#[derive(Clone, Copy)]
pub struct All;

// SAFETY: `reborrow` does not extend access permissions.
unsafe impl AccessScope for All {
    type Borrow<'a>
        = Self
    where
        Self: 'a;

    fn reborrow(&self) -> Self::Borrow<'_> {
        *self
    }

    fn can_read(&self, _id: ComponentId, _components: &Components) -> bool {
        true
    }

    fn can_write(&self, _id: ComponentId, _components: &Components) -> bool {
        true
    }
}

/// [`AccessScope`] that provides access to only the components specified in the
/// provided [`Access`]. [`FilteredEntityRef`] and [`FilteredEntityMut`] use
/// this scope.
///
/// [`FilteredEntityRef`]: crate::world::FilteredEntityRef
/// [`FilteredEntityMut`]: crate::world::FilteredEntityMut
#[derive(Clone, Copy)]
pub struct Filtered<'s>(pub &'s Access);

// SAFETY: `reborrow` does not extend access permissions.
unsafe impl AccessScope for Filtered<'_> {
    type Borrow<'a>
        = Self
    where
        Self: 'a;

    fn reborrow(&self) -> Self::Borrow<'_> {
        *self
    }

    fn can_read(&self, id: ComponentId, _components: &Components) -> bool {
        self.has_component_read(id)
    }

    fn can_write(&self, id: ComponentId, _components: &Components) -> bool {
        self.has_component_write(id)
    }
}

impl<'s, B: Bundle> From<Except<'s, B>> for Filtered<'s> {
    fn from(value: Except<'s, B>) -> Self {
        // SAFETY: We're not discarding the `Except` semantics as long as the
        // `Access` matched the `Bundle` `B`.
        Filtered(value.0)
    }
}

impl Deref for Filtered<'_> {
    type Target = Access;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// [`AccessScope`] that provides access to all components except those in the
/// provided [`Bundle`] `B`. [`EntityRefExcept`] and [`EntityMutExcept`] use
/// this scope.
///
/// [`EntityRefExcept`]: crate::world::EntityRefExcept
/// [`EntityMutExcept`]: crate::world::EntityMutExcept
pub struct Except<'s, B: Bundle>(pub &'s Access, PhantomData<B>);

impl<'s, B: Bundle> Except<'s, B> {
    /// Creates a new `Except` scope from the given [`Access`].
    ///
    /// # Safety
    ///
    /// The provided `Access` must accurately reflect the components in `B`.
    pub unsafe fn new(access: &'s Access) -> Self {
        Except(access, PhantomData)
    }
}

impl<'s, B: Bundle> Copy for Except<'s, B> {}

impl<'s, B: Bundle> Clone for Except<'s, B> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: `reborrow` does not extend access permissions.
unsafe impl<B: Bundle> AccessScope for Except<'_, B> {
    type Borrow<'a>
        = Self
    where
        Self: 'a;

    fn reborrow(&self) -> Self::Borrow<'_> {
        *self
    }

    fn can_read(&self, id: ComponentId, components: &Components) -> bool {
        B::get_component_ids(components)
            .flatten()
            .all(|b_id| b_id != id)
    }

    fn can_write(&self, id: ComponentId, components: &Components) -> bool {
        B::get_component_ids(components)
            .flatten()
            .all(|b_id| b_id != id)
    }
}

impl<B: Bundle> Deref for Except<'_, B> {
    type Target = Access;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::Component;

    use crate::{
        query::Access,
        world::{AccessScope, Except, World},
    };

    #[derive(Component)]
    pub struct TestComponent<const N: usize>;

    #[test]
    fn all() {
        let mut world = World::new();

        let c1 = world.register_component::<TestComponent<1>>();
        let c2 = world.register_component::<TestComponent<2>>();
        let c3 = world.register_component::<TestComponent<3>>();

        let scope = super::All;

        assert!(scope.can_read(c1, world.components()));
        assert!(scope.can_write(c1, world.components()));

        assert!(scope.can_read(c2, world.components()));
        assert!(scope.can_write(c2, world.components()));

        assert!(scope.can_read(c3, world.components()));
        assert!(scope.can_write(c3, world.components()));
    }

    #[test]
    fn filtered() {
        let mut world = World::new();

        let c1 = world.register_component::<TestComponent<1>>();
        let c2 = world.register_component::<TestComponent<2>>();
        let c3 = world.register_component::<TestComponent<3>>();

        let mut access = Access::new();
        access.add_component_read(c1);
        access.add_component_write(c2);

        let scope = super::Filtered(&access);

        assert!(scope.can_read(c1, world.components()));
        assert!(!scope.can_write(c1, world.components()));

        assert!(scope.can_read(c2, world.components()));
        assert!(scope.can_write(c2, world.components()));

        assert!(!scope.can_read(c3, world.components()));
        assert!(!scope.can_write(c3, world.components()));
    }

    #[test]
    fn except() {
        let mut world = World::new();

        let c1 = world.register_component::<TestComponent<1>>();
        let c2 = world.register_component::<TestComponent<2>>();
        let c3 = world.register_component::<TestComponent<3>>();

        let mut access = Access::new_write_all();
        access.add_component_write(c1);
        access.add_component_write(c2);

        // SAFETY: The `Access` accurately reflects the excluded components.
        let scope = unsafe { Except::<(TestComponent<1>, TestComponent<2>)>::new(&access) };

        assert!(!scope.can_read(c1, world.components()));
        assert!(!scope.can_write(c1, world.components()));

        assert!(!scope.can_read(c2, world.components()));
        assert!(!scope.can_write(c2, world.components()));

        assert!(scope.can_read(c3, world.components()));
        assert!(scope.can_write(c3, world.components()));
    }
}
