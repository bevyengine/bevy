use std::{
    any::TypeId,
    cell::RefCell,
    rc::Rc,
    sync::{RwLockReadGuard, RwLockWriteGuard},
};

use bevy_utils::HashMap;

use crate::{
    component::{Component, Components},
    prelude::{Entity, Mut},
    query::{EntityFetch, Fetch, ReadFetch, WithFetch, WithoutFetch, WorldQuery, WriteFetch},
    world::{world_cell::command::CellInsert, CellCommandQueue, WorldOverlay},
};

pub(crate) type FetchRefs = Rc<RefCell<FetchAccess>>;

#[derive(Default)]
pub struct FetchAccess {
    access: HashMap<(u32, TypeId), u32>,
}

const UNIQUE_ACCESS: u32 = 0;
const BASE_ACCESS: u32 = 1;
impl FetchAccess {
    fn get_or_base(&mut self, entity: Entity, id: TypeId) -> &mut u32 {
        self.access.entry((entity.id(), id)).or_insert(BASE_ACCESS)
    }

    fn read(&mut self, entity: Entity, id: TypeId) -> bool {
        let id_access = self.get_or_base(entity, id);
        if *id_access == UNIQUE_ACCESS {
            false
        } else {
            *id_access += 1;
            true
        }
    }

    fn drop_read(&mut self, entity: Entity, id: TypeId) {
        let id_access = self.get_or_base(entity, id);
        *id_access -= 1;
    }

    fn write(&mut self, entity: Entity, id: TypeId) -> bool {
        let id_access = self.get_or_base(entity, id);
        if *id_access == BASE_ACCESS {
            *id_access = UNIQUE_ACCESS;
            true
        } else {
            false
        }
    }

    fn drop_write(&mut self, entity: Entity, id: TypeId) {
        let id_access = self.get_or_base(entity, id);
        *id_access = BASE_ACCESS;
    }
}

pub enum CellRef<'w, T: 'static> {
    World {
        inner: &'w T,
        entity: Entity,
        refs: FetchRefs,
    },
    Overlay {
        guard: RwLockReadGuard<'w, T>,
    },
}

impl<'w, T: 'static> CellRef<'w, T> {
    fn new(inner: &'w T, entity: Entity, refs: &FetchRefs) -> Self {
        if !refs.borrow_mut().read(entity, TypeId::of::<T>()) {
            panic!(
                "Component '{}', of entity {:?} already mutably borrowed",
                std::any::type_name::<T>(),
                entity
            );
        }
        Self::World {
            inner,
            entity,
            refs: refs.clone(),
        }
    }

    fn new_overlay(guard: RwLockReadGuard<'w, T>) -> Self {
        Self::Overlay { guard }
    }
}

impl<'w, T> std::ops::Deref for CellRef<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::World { inner, .. } => inner,
            Self::Overlay { guard } => guard.deref(),
        }
    }
}

impl<'w, T: 'static> Drop for CellRef<'w, T> {
    fn drop(&mut self) {
        match self {
            Self::World { entity, refs, .. } => {
                refs.borrow_mut().drop_read(*entity, TypeId::of::<T>())
            }
            Self::Overlay { .. } => {}
        }
    }
}

pub enum CellMut<'w, T: 'static> {
    World {
        inner: Mut<'w, T>,
        entity: Entity,
        refs: FetchRefs,
    },
    Overlay {
        guard: RwLockWriteGuard<'w, T>,
    },
}

impl<'w, T: 'static> CellMut<'w, T> {
    fn new(inner: Mut<'w, T>, entity: Entity, refs: &FetchRefs) -> Self {
        if !refs.borrow_mut().write(entity, TypeId::of::<T>()) {
            panic!(
                "Component '{}' of entity {:?} already borrowed",
                std::any::type_name::<T>(),
                entity
            );
        }
        Self::World {
            inner,
            entity,
            refs: refs.clone(),
        }
    }

    fn new_overlay(guard: RwLockWriteGuard<'w, T>) -> Self {
        Self::Overlay { guard }
    }
}

impl<'w, T> std::ops::Deref for CellMut<'w, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::World { inner, .. } => inner.deref(),
            Self::Overlay { guard } => guard.deref(),
        }
    }
}

impl<'w, T> std::ops::DerefMut for CellMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::World { inner, .. } => inner.deref_mut(),
            Self::Overlay { guard } => guard.deref_mut(),
        }
    }
}

impl<'w, T: 'static> Drop for CellMut<'w, T> {
    fn drop(&mut self) {
        match self {
            Self::World { entity, refs, .. } => {
                refs.borrow_mut().drop_write(*entity, TypeId::of::<T>())
            }
            Self::Overlay { .. } => {}
        }
    }
}

pub trait WorldCellQuery: WorldQuery {
    type CellFetch: for<'world, 'state> CellFetch<
        'world,
        'state,
        State = Self::State,
        Item = <Self::Fetch as Fetch<'world, 'state>>::Item,
    >;
}

impl<T> WorldCellQuery for T
where
    T: WorldQuery,
    T::Fetch: for<'world, 'state> CellFetch<'world, 'state>,
{
    type CellFetch = T::Fetch;
}

// pub trait CellFilterFetch: FilterFetch + for<'w> CellFetch<'w> {}
// impl<T> CellFilterFetch for T where T: FilterFetch + for<'w> CellFetch<'w> {}

pub trait CellFetch<'world, 'state>: Fetch<'world, 'state> {
    type CellItem;
    // just wrap original data without further filtering
    fn wrap(inner: Self::Item, entity: Entity, refs: &FetchRefs) -> Self::CellItem;

    // wrap original data, perform filtering and replacements using overlay data
    fn overlay(
        inner: Self::Item,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
        command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem>;

    // fetch using combined world and overlay data
    fn fetch_overlay(
        &mut self,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
    ) -> Option<Self::CellItem>;
}

impl<'world, 'state> CellFetch<'world, 'state> for EntityFetch {
    type CellItem = <Self as Fetch<'world, 'state>>::Item;
    fn wrap(inner: Self::Item, _entity: Entity, _refs: &FetchRefs) -> Self::CellItem {
        inner
    }

    fn overlay(
        inner: Self::Item,
        _entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
        _command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem> {
        Some(inner)
    }

    #[inline]
    fn fetch_overlay(
        &mut self,
        entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
    ) -> Option<Self::CellItem> {
        Some(entity)
    }
}

impl<'world, 'state, T: Component> CellFetch<'world, 'state> for WithFetch<T> {
    type CellItem = <Self as Fetch<'world, 'state>>::Item;
    fn wrap(inner: Self::Item, _entity: Entity, _refs: &FetchRefs) -> Self::CellItem {
        inner
    }

    fn overlay(
        inner: Self::Item,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
        _command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem> {
        if let Some(removed) = overlay.removed.get(&entity) {
            let id = components.get_id(TypeId::of::<T>())?;
            if removed.contains(&id) {
                return None;
            }
        }
        Some(Self::wrap(inner, entity, refs))
    }

    fn fetch_overlay(
        &mut self,
        _entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
    ) -> Option<Self::CellItem> {
        todo!()
    }
}

impl<'world, 'state, T: Component> CellFetch<'world, 'state> for WithoutFetch<T> {
    type CellItem = <Self as Fetch<'world, 'state>>::Item;
    fn wrap(inner: Self::Item, _entity: Entity, _refs: &FetchRefs) -> Self::CellItem {
        inner
    }

    fn overlay(
        inner: Self::Item,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
        _command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem> {
        if let Some(inserted) = overlay.inserted.get(&entity) {
            let id = components.get_id(TypeId::of::<T>())?;
            if inserted.iter().find(|i| i.0 == id).is_some() {
                return None;
            }
        }
        Some(Self::wrap(inner, entity, refs))
    }

    fn fetch_overlay(
        &mut self,
        _entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
    ) -> Option<Self::CellItem> {
        todo!()
    }
}

impl<'world, 'state, T: Component> CellFetch<'world, 'state> for ReadFetch<T> {
    type CellItem = CellRef<'world, T>;
    fn wrap(inner: Self::Item, entity: Entity, refs: &FetchRefs) -> Self::CellItem {
        CellRef::new(inner, entity, refs)
    }

    fn overlay(
        inner: Self::Item,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
        command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem> {
        let id = components.get_id(TypeId::of::<T>())?;
        // component removed, filter the result out
        if let Some(removed) = overlay.removed.get(&entity) {
            if removed.contains(&id) {
                return None;
            }
        }
        // component inserted, return a reference to inserted component
        if let Some(inserted) = overlay.inserted.get(&entity) {
            if let Some((_, cmd_id)) = inserted.iter().find(|i| i.0 == id) {
                let cmd = unsafe { command_queue.get_nth::<CellInsert<T>>(*cmd_id) };
                let guard = cmd.component.try_read().expect("already borrowed");
                return Some(CellRef::new_overlay(guard));
            }
        }

        Some(Self::wrap(inner, entity, refs))
    }

    fn fetch_overlay(
        &mut self,
        _entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
    ) -> Option<Self::CellItem> {
        todo!()
    }
}

impl<'world, 'state, T: Component> CellFetch<'world, 'state> for WriteFetch<T> {
    type CellItem = CellMut<'world, T>;
    fn wrap(inner: Self::Item, entity: Entity, refs: &FetchRefs) -> Self::CellItem {
        CellMut::new(inner, entity, refs)
    }

    fn overlay(
        inner: Self::Item,
        entity: Entity,
        refs: &FetchRefs,
        overlay: &WorldOverlay,
        components: &Components,
        command_queue: &'world CellCommandQueue,
    ) -> Option<Self::CellItem> {
        let id = components.get_id(TypeId::of::<T>())?;
        // component removed, filter the result out
        if let Some(removed) = overlay.removed.get(&entity) {
            if removed.contains(&id) {
                return None;
            }
        }
        // component inserted, return a reference to inserted component
        if let Some(inserted) = overlay.inserted.get(&entity) {
            if let Some((_, cmd_id)) = inserted.iter().find(|i| i.0 == id) {
                let cmd = unsafe { command_queue.get_nth::<CellInsert<T>>(*cmd_id) };
                let guard = cmd.component.try_write().expect("already borrowed");
                return Some(CellMut::new_overlay(guard));
            }
        }

        Some(Self::wrap(inner, entity, refs))
    }

    fn fetch_overlay(
        &mut self,
        _entity: Entity,
        _refs: &FetchRefs,
        _overlay: &WorldOverlay,
        _components: &Components,
    ) -> Option<Self::CellItem> {
        todo!()
    }
}

macro_rules! impl_tuple_cell_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(non_snake_case)]
        impl<'world, 'state, $($name: CellFetch<'world, 'state>),*> CellFetch<'world, 'state> for ($($name,)*) {
            type CellItem = ($($name::CellItem,)*);

            #[allow(clippy::unused_unit)]
            #[inline]
            fn wrap(inner: Self::Item, _entity: Entity, _refs: &FetchRefs) -> Self::CellItem {
                let ($($name,)*) = inner;
                ($(<$name as CellFetch<'world, 'state>>::wrap($name, _entity, _refs),)*)
            }

            #[inline]
            fn overlay(
                inner: Self::Item,
                _entity: Entity,
                _refs: &FetchRefs,
                _overlay: &WorldOverlay,
                _components: &Components,
                _command_queue: &'world CellCommandQueue,
            ) -> Option<Self::CellItem> {
                let ($($name,)*) = inner;
                Some(($(<$name as CellFetch<'world, 'state>>::overlay($name, _entity, _refs, _overlay, _components, _command_queue)?,)*))
            }

            #[inline]
            fn fetch_overlay(
                &mut self,
                _entity: Entity,
                _refs: &FetchRefs,
                _overlay: &WorldOverlay,
                _components: &Components,
            ) -> Option<Self::CellItem> {
                let ($(ref mut $name,)*) = self;
                Some(($(<$name as CellFetch<'world, 'state>>::fetch_overlay($name, _entity, _refs, _overlay, _components)?,)*))
            }
        }
    };
}

bevy_ecs_macros::all_tuples!(impl_tuple_cell_fetch, 0, 15, F, S);
