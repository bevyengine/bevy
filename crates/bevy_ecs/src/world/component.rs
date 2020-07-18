use crate::{Archetype, Component, HecsQuery};
use hecs::{Access, Fetch};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

/// Unique borrow of an entity's component
pub struct ComMut<'a, T: Component> {
    value: &'a mut T,
    modified: &'a mut bool,
}

unsafe impl<T: Component> Send for ComMut<'_, T> {}
unsafe impl<T: Component> Sync for ComMut<'_, T> {}

impl<'a, T: Component> Deref for ComMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.value
    }
}

impl<'a, T: Component> DerefMut for ComMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        *self.modified = true;
        self.value
    }
}

impl<'a, T: Component> HecsQuery for ComMut<'a, T> {
    type Fetch = FetchComMut<T>;
}
#[doc(hidden)]
pub struct FetchComMut<T>(NonNull<T>, NonNull<bool>);

impl<'a, T: Component> Fetch<'a> for FetchComMut<T> {
    type Item = ComMut<'a, T>;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            Some(Access::Write)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        archetype.borrow_mut::<T>();
    }
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        archetype
            .get_with_modified::<T>()
            .map(|(components, modified)| {
                Self(
                    NonNull::new_unchecked(components.as_ptr().add(offset)),
                    NonNull::new_unchecked(modified.as_ptr().add(offset)),
                )
            })
    }
    fn release(archetype: &Archetype) {
        archetype.release_mut::<T>();
    }

    unsafe fn next(&mut self) -> ComMut<'a, T> {
        let component = self.0.as_ptr();
        let modified = self.1.as_ptr();
        self.0 = NonNull::new_unchecked(component.add(1));
        self.1 = NonNull::new_unchecked(modified.add(1));
        ComMut {
            value: &mut *component,
            modified: &mut *modified,
        }
    }
}

pub struct Changed<T, Q>(PhantomData<(Q, fn(T))>);

impl<T: Component, Q: HecsQuery> HecsQuery for Changed<T, Q> {
    type Fetch = FetchChanged<T, Q::Fetch>;
}

#[doc(hidden)]
pub struct FetchChanged<T, F>(F, PhantomData<fn(T)>, NonNull<bool>);

impl<'a, T: Component, F: Fetch<'a>> Fetch<'a> for FetchChanged<T, F> {
    type Item = F::Item;

    fn access(archetype: &Archetype) -> Option<Access> {
        if archetype.has::<T>() {
            F::access(archetype)
        } else {
            None
        }
    }

    fn borrow(archetype: &Archetype) {
        F::borrow(archetype)
    }
    unsafe fn get(archetype: &'a Archetype, offset: usize) -> Option<Self> {
        if !archetype.has::<T>() {
            return None;
        }
        Some(Self(
            F::get(archetype, offset)?,
            PhantomData,
            NonNull::new_unchecked(archetype.get_modified::<T>()?.as_ptr().add(offset)),
        ))
    }
    fn release(archetype: &Archetype) {
        F::release(archetype)
    }

    unsafe fn should_skip(&self) -> bool {
        // skip if the current item wasn't changed
        !*self.2.as_ref()
    }

    unsafe fn next(&mut self) -> F::Item {
        self.2 = NonNull::new_unchecked(self.2.as_ptr().add(1));
        self.0.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Changed, ComMut};
    use hecs::{Entity, World};

    struct A(usize);
    struct B;
    struct C;

    #[test]
    fn modified_trackers() {
        let mut world = World::default();
        let e1 = world.spawn((A(0), B));
        let e2 = world.spawn((A(0), B));
        let e3 = world.spawn((A(0), B));
        world.spawn((A(0), B));

        for (i, mut a) in world.query::<ComMut<A>>().iter().enumerate() {
            if i % 2 == 0 {
                a.0 += 1;
            }
        }

        fn get_changed(world: &World) -> Vec<Entity> {
            world
                .query::<Changed<A, Entity>>()
                .iter()
                .collect::<Vec<Entity>>()
        };

        assert_eq!(get_changed(&world), vec![e1, e3]);

        // ensure changing an entity's archetypes also moves its modified state
        world.insert(e1, (C,)).unwrap();

        assert_eq!(get_changed(&world), vec![e3, e1], "changed entities list should not change (although the order will due to archetype moves)");

        // spawning a new A entity should not change existing modified state
        world.insert(e1, (A(0), B)).unwrap();
        assert_eq!(get_changed(&world), vec![e3, e1], "changed entities list should not change");

        // removing an unchanged entity should not change modified state
        world.despawn(e2).unwrap();
        assert_eq!(get_changed(&world), vec![e3, e1], "changed entities list should not change");

        // removing a changed entity should remove it from enumeration
        world.despawn(e1).unwrap();
        assert_eq!(get_changed(&world), vec![e3], "e1 should no longer be returned");

        world.clear_trackers();

        assert!(world
            .query::<Changed<A, Entity>>()
            .iter()
            .collect::<Vec<Entity>>()
            .is_empty());
    }
}
