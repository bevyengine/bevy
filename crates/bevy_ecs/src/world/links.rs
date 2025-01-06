//! This modules defines and implements links between worlds.
//! This allows a system to have [`SystemParam`]s that link to other worlds besides the world the system lives in.

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    archetype::ArchetypeGeneration,
    prelude::DetectChangesMut,
    system::{SystemMeta, SystemParam, SystemParamItem},
};

use super::{unsafe_world_cell::UnsafeWorldCell, Mut, World, WorldId};

/// This trait allows a link to be made between a two worlds via a system. One world can hold a nested world via this link.
pub trait WorldLink: Send + Sync + 'static {
    /// gets the linked world mutably
    fn get_world_mut(&mut self) -> &mut World;
    /// gets the linked world as an unsafe cell.
    ///
    /// # Safety
    /// It is up to the caller to ensure the cell is used responsibly and does not conflict with any other access to the link.
    unsafe fn get_unsafe_world(&self) -> UnsafeWorldCell;
}

/// this is purely used to facilitate the resource derive
mod wrapper {
    use super::WorldLink;
    use crate as bevy_ecs;
    use bevy_ecs_macros::Resource;

    /// This resource links one world to another via a [`WorldLink`].
    ///
    /// # Safety
    ///
    /// This resource is private to this file. Hence, there can be no conflicts outside of this file. To prevent conflicts in the file,
    /// access should be purely read-only, unless exclusive access to the world is guaranteed. Note: These rules would break if nested exclusive access were possible
    /// (exclusive access is what allows you to edit a link, which may effect [`SystemParam::update_meta`](super::SystemParam::update_meta)),
    /// but since `&mut World` does not implement `SystemParam`, this is not possible. To get nested exclusive access, you need exclusive access to the root world.
    /// This prevents these conflicts.
    #[derive(Resource)]
    pub(super) struct Link<L: WorldLink>(pub L);
}
use wrapper::Link;

/// A [`SystemParam`] that is sourced from a linked world via an internal link resource.
pub struct Linked<'w, 's, L: WorldLink, P: SystemParam> {
    /// the parameter item requested
    item: SystemParamItem<'w, 's, P>,
    marker: PhantomData<L>,
}

impl<'w, 's, L: WorldLink + DerefMut, P: SystemParam + DerefMut> DerefMut for Linked<'w, 's, L, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<'w, 's, L: WorldLink + Deref, P: SystemParam + Deref> Deref for Linked<'w, 's, L, P> {
    type Target = SystemParamItem<'w, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}

// SAFETY: This assumes the Link exists, pannicing otherwise. This assumes that the inner system parameter will report access properly.
unsafe impl<L: WorldLink, P: SystemParam> SystemParam for Linked<'_, '_, L, P> {
    // the WorldId is for the linked world
    type State = (P::State, WorldId, ArchetypeGeneration);

    type Item<'world, 'state> = Linked<'world, 'state, L, P>;

    fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
        // see See [`Link`]. for why future access doesn't need to be registered
        world.register_resource::<Link<L>>();

        let mut link = world
            .get_resource_mut::<Link<L>>()
            .expect("attempting to build a system parameter from a missing link");

        let link_world = link.bypass_change_detection().0.get_world_mut();
        let state = P::init_state(link_world, system_meta);
        (state, link_world.id(), link_world.archetypes().generation())
    }

    unsafe fn get_param<'world, 'state>(
        state: &'state mut Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell<'world>,
        _change_tick: crate::component::Tick,
    ) -> Self::Item<'world, 'state> {
        // Safety: See [`Link`].
        let link = unsafe {
            world
                .get_resource::<Link<L>>()
                .expect("attempting to get a system parameter from a missing link")
        };
        // Safety: See [`Link`].
        let link = unsafe { link.0.get_unsafe_world() };
        // prevents undefined behavior. If this ever fails, the safety of this system param is compromised.
        assert_eq!(state.1, link.id());

        // IMPORTANT: We use the link's change tick to keep it fully separate from the base world. This is what is really relevant.
        let item = P::get_param(&mut state.0, system_meta, link, link.change_tick());
        Linked {
            item,
            marker: PhantomData,
        }
    }

    unsafe fn update_meta(
        state: &mut Self::State,
        world: UnsafeWorldCell,
        system_meta: &mut SystemMeta,
    ) {
        // Safety: See [`Link`].
        let link = unsafe {
            world
                .get_resource::<Link<L>>()
                .expect("attempting to get a system parameter from a missing link")
        };
        // Safety: See [`Link`].
        let link = unsafe { link.0.get_unsafe_world() };
        // prevents undefined behavior. If this ever fails, the safety of this system param is compromised.
        assert_eq!(state.1, link.id());

        let archetypes = link.archetypes();
        let old_generation = core::mem::replace(&mut state.2, archetypes.generation());
        for archetype in &archetypes[old_generation..] {
            // SAFETY: The assertion above ensures that the param_state was initialized from `link`.
            unsafe { P::new_archetype(&mut state.0, archetype, system_meta) };
        }

        P::update_meta(&mut state.0, link, system_meta);
    }

    fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
        let mut link = world
            .get_resource_mut::<Link<L>>()
            .expect("attempting to apply a system parameter from a missing link");
        P::apply(
            &mut state.0,
            system_meta,
            link.bypass_change_detection().0.get_world_mut(),
        );
    }

    fn queue(state: &mut Self::State, system_meta: &SystemMeta, world: super::DeferredWorld) {
        let link = world
            .get_resource::<Link<L>>()
            .expect("attempting to queue a system parameter from a missing link");
        // Safety: See [`Link`].
        let link = unsafe { link.0.get_unsafe_world() };
        // Safety: `link` has not been used to get any mutably references
        let link_deferred = unsafe { link.into_deferred() };
        P::queue(&mut state.0, system_meta, link_deferred);
    }

    unsafe fn validate_param(
        state: &Self::State,
        system_meta: &SystemMeta,
        world: UnsafeWorldCell,
    ) -> bool {
        let Some(link) = world.get_resource::<Link<L>>() else {
            return false;
        };
        // Safety: See [`Link::inner`]
        P::validate_param(&state.0, system_meta, unsafe { link.0.get_unsafe_world() })
    }
}

/// Allows viewing a link
pub struct LinkPeek<'w, L: WorldLink>(Mut<'w, Link<L>>);

impl World {
    /// Links this world to another one. This is one way. You can use the link in systems via [`Linked`]
    pub fn link<L: WorldLink>(&mut self, link: L) {
        self.insert_resource(Link(link));
    }

    /// Removes the world link of this type if it was linked. Note that if a system with [`Linked`] depends on this and is run, it will panic.
    pub fn unlink<L: WorldLink>(&mut self) {
        self.remove_resource::<Link<L>>();
    }

    // NOTE: we can't safely provide an api for this that takes an immutable frerence to world. See [`Link`] for why.
    /// Allows modification of an active world link. Returns `None` if the link was not active.
    pub fn peek_link<'w, L: WorldLink>(&'w mut self) -> Option<LinkPeek<'w, L>> {
        self.get_resource_mut::<Link<L>>().map(LinkPeek)
    }
}

impl<L: WorldLink> Deref for LinkPeek<'_, L> {
    type Target = L;

    fn deref(&self) -> &Self::Target {
        &self.0 .0
    }
}

impl<L: WorldLink> DerefMut for LinkPeek<'_, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0 .0
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_ecs, prelude::Schedule, system::RunSystemOnce};
    use bevy_ecs_macros::{Component, Resource};

    use crate::system::{Query, ResMut};

    use super::*;

    struct NestedWorld(World);

    #[derive(Resource)]
    struct MyRes;

    #[derive(Component)]
    struct MyComponent;

    impl WorldLink for NestedWorld {
        fn get_world_mut(&mut self) -> &mut World {
            &mut self.0
        }

        unsafe fn get_unsafe_world(&self) -> UnsafeWorldCell {
            UnsafeWorldCell::new_readonly(&self.0)
        }
    }

    #[test]
    fn test_example_usage() {
        let mut main = World::new();
        main.insert_resource(MyRes);
        main.link(NestedWorld(World::new()));
        main.peek_link::<NestedWorld>()
            .unwrap()
            .deref_mut()
            .0
            .insert_resource(MyRes);

        fn my_system(
            _main_res: ResMut<MyRes>,
            _main_query: Query<&mut MyComponent>,
            _nested_res: Linked<NestedWorld, ResMut<MyRes>>,
            _nested_query: Linked<NestedWorld, Query<&'static mut MyComponent>>,
        ) {
            // would do stuff here
        }

        main.run_system_once(my_system).unwrap();

        let mut schedule = Schedule::default();
        schedule.add_systems(my_system);
        schedule.run(&mut main);
    }
}
