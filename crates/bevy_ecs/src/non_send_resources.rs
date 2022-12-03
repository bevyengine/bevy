use std::{
    any::TypeId,
    cell::RefCell,
    marker::PhantomData,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

use crate as bevy_ecs;
use bevy_ptr::{OwningPtr, Ptr};
use bevy_tasks::ThreadExecutor;

use crate::{
    archetype::ArchetypeComponentId,
    change_detection::{Mut, Ticks},
    component::{ComponentId, Components, TickCells},
    storage::{ResourceData, Resources},
    system::Resource,
};

thread_local! {
    pub static NON_SEND_RESOURCES: RefCell<NonSendResources> = RefCell::new(NonSendResources::new());
}

pub struct NonSendResources {
    components: Components,
    non_send_resources: Resources,
    archetype_component_count: usize,
    change_tick: AtomicU32,
    last_change_tick: u32,
    _not_send_sync: PhantomData<*const ()>,
}

impl Default for NonSendResources {
    fn default() -> Self {
        Self {
            components: Default::default(),
            non_send_resources: Resources::default(),
            archetype_component_count: 0,
            // Default value is `1`, and `last_change_tick`s default to `0`, such that changes
            // are detected on first system runs and for direct world queries.
            change_tick: AtomicU32::new(1),
            last_change_tick: 0,
            _not_send_sync: PhantomData::default(),
        }
    }
}

impl NonSendResources {
    pub fn new() -> Self {
        Self::default()
    }

    /// # Safety
    /// `component_id` must be valid for this world
    #[inline]
    unsafe fn initialize_resource_internal(
        &mut self,
        component_id: ComponentId,
    ) -> &mut ResourceData {
        let archetype_component_count = &mut self.archetype_component_count;
        self.non_send_resources
            .initialize_with(component_id, &self.components, || {
                let id = ArchetypeComponentId::new(*archetype_component_count);
                *archetype_component_count += 1;
                id
            })
    }

    // Note: used by SystemParam to initialize the resource
    // pub(crate) fn initialize_resource<R: 'static>(&mut self) -> ComponentId {
    //     let component_id = self.components.init_non_send::<R>();
    //     // SAFETY: resource initialized above
    //     unsafe { self.initialize_resource_internal(component_id) };
    //     component_id
    // }

    #[inline]
    pub fn init_resource<R: 'static + Default>(&mut self) {
        if !self.contains_resource::<R>() {
            let resource = R::default();
            self.insert_resource(resource);
        }
    }

    #[inline]
    pub fn insert_resource<R: 'static>(&mut self, value: R) {
        let component_id = self.components.init_non_send::<R>();
        OwningPtr::make(value, |ptr| {
            // SAFETY: component_id was just initialized and corresponds to resource of type R
            unsafe {
                self.insert_resource_by_id(component_id, ptr);
            }
        });
    }

    /// Removes the resource of a given type and returns it, if it exists. Otherwise returns [None].
    #[inline]
    pub fn remove_resource<R: 'static>(&mut self) -> Option<R> {
        // SAFETY: R is Send + Sync
        unsafe { self.remove_resource_unchecked() }
    }

    #[inline]
    /// # Safety
    /// Only remove `NonSend` resources from the main thread
    /// as they cannot be sent across threads
    #[allow(unused_unsafe)]
    pub unsafe fn remove_resource_unchecked<R: 'static>(&mut self) -> Option<R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: the resource is of type R and the value is returned back to the caller.
        unsafe {
            let (ptr, _) = self.non_send_resources.get_mut(component_id)?.remove()?;
            Some(ptr.read::<R>())
        }
    }

    #[inline]
    pub fn contains_resource<R: 'static>(&self) -> bool {
        self.components
            .get_resource_id(TypeId::of::<R>())
            .and_then(|component_id| self.non_send_resources.get(component_id))
            .map(|info| info.is_present())
            .unwrap_or(false)
    }

    /// # Safety
    /// The value referenced by `value` must be valid for the given [`ComponentId`] of this world
    /// `component_id` must exist in this [`World`]
    #[inline]
    pub unsafe fn insert_resource_by_id(
        &mut self,
        component_id: ComponentId,
        value: OwningPtr<'_>,
    ) {
        let change_tick = self.change_tick();

        // SAFETY: component_id is valid, ensured by caller
        self.initialize_resource_internal(component_id)
            .insert(value, change_tick);
    }

    #[inline]
    pub fn change_tick(&mut self) -> u32 {
        *self.change_tick.get_mut()
    }

    #[inline]
    #[track_caller]
    pub fn resource<R: 'static>(&self) -> &R {
        match self.get_resource() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    #[inline]
    #[track_caller]
    pub fn resource_mut<R: 'static>(&mut self) -> Mut<'_, R> {
        match self.get_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<R>()
            ),
        }
    }

    /// Gets a reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource<R: 'static>(&self) -> Option<&R> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: unique world access
        unsafe { self.get_resource_with_id(component_id) }
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource_mut<R: 'static>(&mut self) -> Option<Mut<'_, R>> {
        // SAFETY: unique world access
        unsafe { self.get_resource_unchecked_mut() }
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    #[inline]
    pub(crate) unsafe fn get_resource_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<&R> {
        self.non_send_resources
            .get(component_id)?
            .get_data()
            .map(|ptr| ptr.deref())
    }

    /// # Safety
    /// This will allow aliased mutable access to the given resource type. The caller must ensure
    /// that there is either only one mutable access or multiple immutable accesses at a time.
    #[inline]
    pub unsafe fn get_resource_unchecked_mut<R: 'static>(&self) -> Option<Mut<'_, R>> {
        let component_id = self.components.get_resource_id(TypeId::of::<R>())?;
        self.get_resource_unchecked_mut_with_id(component_id)
    }

    /// # Safety
    /// `component_id` must be assigned to a component of type `R`
    /// Caller must ensure this doesn't violate Rust mutability rules for the given resource.
    #[inline]
    pub(crate) unsafe fn get_resource_unchecked_mut_with_id<R>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'_, R>> {
        let (ptr, ticks) = self.get_resource_with_ticks(component_id)?;
        Some(Mut {
            value: ptr.assert_unique().deref_mut(),
            ticks: Ticks::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick()),
        })
    }

    // Shorthand helper function for getting the data and change ticks for a resource.
    #[inline]
    pub(crate) fn get_resource_with_ticks(
        &self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'_>, TickCells<'_>)> {
        self.non_send_resources.get(component_id)?.get_with_ticks()
    }

    #[inline]
    pub fn last_change_tick(&self) -> u32 {
        self.last_change_tick
    }

    #[inline]
    pub fn increment_change_tick(&self) -> u32 {
        self.change_tick.fetch_add(1, Ordering::AcqRel)
    }

    #[inline]
    pub fn read_change_tick(&self) -> u32 {
        self.change_tick.load(Ordering::Acquire)
    }

    // TODO: make change ticks work!
}

/// New-typed [`ThreadExecutor`] [`Resource`] that is used to run systems on the main thread
#[derive(Resource, Default)]
pub struct MainThreadExecutor(pub Arc<ThreadExecutor>);

impl MainThreadExecutor {
    pub fn new() -> Self {
        MainThreadExecutor(Arc::new(ThreadExecutor::new()))
    }

    /// run a FnMut on the main thread with the nonsend resources
    pub fn run<R: Send>(&self, mut f: impl FnMut(&mut NonSendResources) -> R + Send) -> R {
        // TODO: check if we're on the correct thread and just run the function inline if we are
        self.0.spawner().block_on(async move {
            NON_SEND_RESOURCES.with(|non_send_resources| f(&mut non_send_resources.borrow_mut()))
        })
    }
}

impl Clone for MainThreadExecutor {
    fn clone(&self) -> Self {
        MainThreadExecutor(self.0.clone())
    }
}
