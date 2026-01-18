use core::{marker::PhantomData, ops::Deref};

use crate::{bundle::Bundle, query::Access};

/// Defines the set of [`Component`]s accessible by the entity reference types
/// [`EntityRef`] and [`EntityMut`].
///
/// The following accesses are provided:
/// - [`All`]: Provides access to all components. This is the default access.
/// - [`Filtered`]: Provides access only to the components specified in an
///   [`Access`]. This is used by [`FilteredEntityRef`] and [`FilteredEntityMut`].
/// - [`Except`]: Provides access to all components except those in a specified
///   [`Bundle`]. This is used by [`EntityRefExcept`] and [`EntityMutExcept`].
///
/// [`Component`]: crate::component::Component
/// [`EntityRef`]: crate::world::EntityRef
/// [`EntityMut`]: crate::world::EntityMut
/// [`FilteredEntityRef`]: crate::world::FilteredEntityRef
/// [`FilteredEntityMut`]: crate::world::FilteredEntityMut
/// [`EntityRefExcept`]: crate::world::EntityRefExcept
/// [`EntityMutExcept`]: crate::world::EntityMutExcept
pub trait AsAccess: Copy + Deref<Target = Access> {}

/// [`AsAccess`] that provides access to all of an entity's components. This
/// is the default access of [`EntityRef`] and [`EntityMut`].
///
/// [`EntityRef`]: crate::world::EntityRef
/// [`EntityMut`]: crate::world::EntityMut
#[derive(Clone, Copy)]
pub struct All;

impl AsAccess for All {}

impl Deref for All {
    type Target = Access;

    fn deref(&self) -> &Access {
        static WRITE_ALL: Access = Access::new_write_all();
        &WRITE_ALL
    }
}

/// [`AsAccess`] that provides access to only the components specified in the
/// provided [`Access`]. [`FilteredEntityRef`] and [`FilteredEntityMut`] use
/// this access.
///
/// [`FilteredEntityRef`]: crate::world::FilteredEntityRef
/// [`FilteredEntityMut`]: crate::world::FilteredEntityMut
#[derive(Clone, Copy)]
pub struct Filtered<'s>(pub &'s Access);

impl AsAccess for Filtered<'_> {}

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

/// [`AsAccess`] that provides access to all components except those in the
/// provided [`Bundle`] `B`. [`EntityRefExcept`] and [`EntityMutExcept`] use
/// this access.
///
/// [`EntityRefExcept`]: crate::world::EntityRefExcept
/// [`EntityMutExcept`]: crate::world::EntityMutExcept
pub struct Except<'s, B: Bundle>(pub &'s Access, PhantomData<B>);

impl<'s, B: Bundle> Except<'s, B> {
    /// Creates a new `Except` access from the given [`Access`].
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

impl<B: Bundle> AsAccess for Except<'_, B> {}

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
        world::{Except, World},
    };

    #[derive(Component)]
    pub struct TestComponent<const N: usize>;

    #[test]
    fn all() {
        let mut world = World::new();

        let c1 = world.register_component::<TestComponent<1>>();
        let c2 = world.register_component::<TestComponent<2>>();
        let c3 = world.register_component::<TestComponent<3>>();

        let all = super::All;

        assert!(all.has_component_read(c1));
        assert!(all.has_component_write(c1));

        assert!(all.has_component_read(c2));
        assert!(all.has_component_write(c2));

        assert!(all.has_component_read(c3));
        assert!(all.has_component_write(c3));
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

        let filtered = super::Filtered(&access);

        assert!(filtered.has_component_read(c1));
        assert!(!filtered.has_component_write(c1));

        assert!(filtered.has_component_read(c2));
        assert!(filtered.has_component_write(c2));

        assert!(!filtered.has_component_read(c3));
        assert!(!filtered.has_component_write(c3));
    }

    #[test]
    fn except() {
        let mut world = World::new();

        let c1 = world.register_component::<TestComponent<1>>();
        let c2 = world.register_component::<TestComponent<2>>();
        let c3 = world.register_component::<TestComponent<3>>();

        let mut access = Access::new_write_all();
        access.remove_component_read(c1);
        access.remove_component_write(c1);
        access.remove_component_read(c2);
        access.remove_component_write(c2);

        // SAFETY: The `Access` accurately reflects the excluded components.
        let except = unsafe { Except::<(TestComponent<1>, TestComponent<2>)>::new(&access) };

        assert!(!except.has_component_read(c1));
        assert!(!except.has_component_write(c1));

        assert!(!except.has_component_read(c2));
        assert!(!except.has_component_write(c2));

        assert!(except.has_component_read(c3));
        assert!(except.has_component_write(c3));
    }
}
