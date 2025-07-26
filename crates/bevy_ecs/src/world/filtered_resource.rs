use crate::{
    change_detection::{Mut, MutUntyped, Ref, Ticks, TicksMut},
    component::{ComponentId, Tick},
    query::Access,
    resource::Resource,
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_ptr::{Ptr, UnsafeCellDeref};

use super::error::ResourceFetchError;

/// Provides read-only access to a set of [`Resource`]s defined by the contained [`Access`].
///
/// Use [`FilteredResourcesMut`] if you need mutable access to some resources.
///
/// To be useful as a [`SystemParam`](crate::system::SystemParam),
/// this must be configured using a [`FilteredResourcesParamBuilder`](crate::system::FilteredResourcesParamBuilder)
/// to build the system using a [`SystemParamBuilder`](crate::prelude::SystemParamBuilder).
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # #[derive(Default, Resource)]
/// # struct B;
/// #
/// # #[derive(Default, Resource)]
/// # struct C;
/// #
/// # let mut world = World::new();
/// // Use `FilteredResourcesParamBuilder` to declare access to resources.
/// let system = (FilteredResourcesParamBuilder::new(|builder| {
///     builder.add_read::<B>().add_read::<C>();
/// }),)
///     .build_state(&mut world)
///     .build_system(resource_system);
///
/// world.init_resource::<A>();
/// world.init_resource::<C>();
///
/// fn resource_system(res: FilteredResources) {
///     // The resource exists, but we have no access, so we can't read it.
///     assert!(res.get::<A>().is_err());
///     // The resource doesn't exist, so we can't read it.
///     assert!(res.get::<B>().is_err());
///     // The resource exists and we have access, so we can read it.
///     let c = res.get::<C>().unwrap();
///     // The type parameter can be left out if it can be determined from use.
///     let c: Ref<C> = res.get().unwrap();
/// }
/// #
/// # world.run_system_once(system);
/// ```
///
/// This can be used alongside ordinary [`Res`](crate::system::Res) and [`ResMut`](crate::system::ResMut) parameters if they do not conflict.
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # #[derive(Default, Resource)]
/// # struct B;
/// #
/// # let mut world = World::new();
/// # world.init_resource::<A>();
/// # world.init_resource::<B>();
/// #
/// let system = (
///     FilteredResourcesParamBuilder::new(|builder| {
///         builder.add_read::<A>();
///     }),
///     ParamBuilder,
///     ParamBuilder,
/// )
///     .build_state(&mut world)
///     .build_system(resource_system);
///
/// // Read access to A does not conflict with read access to A or write access to B.
/// fn resource_system(filtered: FilteredResources, res_a: Res<A>, res_mut_b: ResMut<B>) {
///     let res_a_2: Ref<A> = filtered.get::<A>().unwrap();
/// }
/// #
/// # world.run_system_once(system);
/// ```
///
/// But it will conflict if it tries to read the same resource that another parameter writes.
///
/// ```should_panic
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.init_resource::<A>();
/// #
/// let system = (
///     FilteredResourcesParamBuilder::new(|builder| {
///         builder.add_read::<A>();
///     }),
///     ParamBuilder,
/// )
///     .build_state(&mut world)
///     .build_system(invalid_resource_system);
///
/// // Read access to A conflicts with write access to A.
/// fn invalid_resource_system(filtered: FilteredResources, res_mut_a: ResMut<A>) { }
/// #
/// # world.run_system_once(system);
/// ```
#[derive(Clone, Copy)]
pub struct FilteredResources<'w, 's> {
    world: UnsafeWorldCell<'w>,
    access: &'s Access,
    last_run: Tick,
    this_run: Tick,
}

impl<'w, 's> FilteredResources<'w, 's> {
    /// Creates a new [`FilteredResources`].
    /// # Safety
    /// It is the callers responsibility to ensure that nothing else may access the any resources in the `world` in a way that conflicts with `access`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        access: &'s Access,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            world,
            access,
            last_run,
            this_run,
        }
    }

    /// Returns a reference to the underlying [`Access`].
    pub fn access(&self) -> &Access {
        self.access
    }

    /// Returns `true` if the `FilteredResources` has access to the given resource.
    /// Note that [`Self::get()`] may still return `Err` if the resource does not exist.
    pub fn has_read<R: Resource>(&self) -> bool {
        let component_id = self.world.components().resource_id::<R>();
        component_id.is_some_and(|component_id| self.access.has_resource_read(component_id))
    }

    /// Gets a reference to the resource of the given type if it exists and the `FilteredResources` has access to it.
    pub fn get<R: Resource>(&self) -> Result<Ref<'w, R>, ResourceFetchError> {
        let component_id = self
            .world
            .components()
            .valid_resource_id::<R>()
            .ok_or(ResourceFetchError::NotRegistered)?;
        if !self.access.has_resource_read(component_id) {
            return Err(ResourceFetchError::NoResourceAccess(component_id));
        }

        // SAFETY: We have read access to this resource
        let (value, ticks, caller) = unsafe { self.world.get_resource_with_ticks(component_id) }
            .ok_or(ResourceFetchError::DoesNotExist(component_id))?;

        Ok(Ref {
            // SAFETY: `component_id` was obtained from the type ID of `R`.
            value: unsafe { value.deref() },
            // SAFETY: We have read access to the resource, so no mutable reference can exist.
            ticks: unsafe { Ticks::from_tick_cells(ticks, self.last_run, self.this_run) },
            // SAFETY: We have read access to the resource, so no mutable reference can exist.
            changed_by: unsafe { caller.map(|caller| caller.deref()) },
        })
    }

    /// Gets a pointer to the resource with the given [`ComponentId`] if it exists and the `FilteredResources` has access to it.
    pub fn get_by_id(&self, component_id: ComponentId) -> Result<Ptr<'w>, ResourceFetchError> {
        if !self.access.has_resource_read(component_id) {
            return Err(ResourceFetchError::NoResourceAccess(component_id));
        }
        // SAFETY: We have read access to this resource
        unsafe { self.world.get_resource_by_id(component_id) }
            .ok_or(ResourceFetchError::DoesNotExist(component_id))
    }
}

impl<'w, 's> From<FilteredResourcesMut<'w, 's>> for FilteredResources<'w, 's> {
    fn from(resources: FilteredResourcesMut<'w, 's>) -> Self {
        // SAFETY:
        // - `FilteredResourcesMut` guarantees exclusive access to all resources in the new `FilteredResources`.
        unsafe {
            FilteredResources::new(
                resources.world,
                resources.access,
                resources.last_run,
                resources.this_run,
            )
        }
    }
}

impl<'w, 's> From<&'w FilteredResourcesMut<'_, 's>> for FilteredResources<'w, 's> {
    fn from(resources: &'w FilteredResourcesMut<'_, 's>) -> Self {
        // SAFETY:
        // - `FilteredResourcesMut` guarantees exclusive access to all components in the new `FilteredResources`.
        unsafe {
            FilteredResources::new(
                resources.world,
                resources.access,
                resources.last_run,
                resources.this_run,
            )
        }
    }
}

impl<'w> From<&'w World> for FilteredResources<'w, 'static> {
    fn from(value: &'w World) -> Self {
        const READ_ALL_RESOURCES: &Access = {
            const ACCESS: Access = {
                let mut access = Access::new();
                access.read_all_resources();
                access
            };
            &ACCESS
        };

        let last_run = value.last_change_tick();
        let this_run = value.read_change_tick();
        // SAFETY: We have a reference to the entire world, so nothing else can alias with read access to all resources.
        unsafe {
            Self::new(
                value.as_unsafe_world_cell_readonly(),
                READ_ALL_RESOURCES,
                last_run,
                this_run,
            )
        }
    }
}

impl<'w> From<&'w mut World> for FilteredResources<'w, 'static> {
    fn from(value: &'w mut World) -> Self {
        Self::from(&*value)
    }
}

/// Provides mutable access to a set of [`Resource`]s defined by the contained [`Access`].
///
/// Use [`FilteredResources`] if you only need read-only access to resources.
///
/// To be useful as a [`SystemParam`](crate::system::SystemParam),
/// this must be configured using a [`FilteredResourcesMutParamBuilder`](crate::system::FilteredResourcesMutParamBuilder)
/// to build the system using a [`SystemParamBuilder`](crate::prelude::SystemParamBuilder).
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # #[derive(Default, Resource)]
/// # struct B;
/// #
/// # #[derive(Default, Resource)]
/// # struct C;
/// #
/// # #[derive(Default, Resource)]
/// # struct D;
/// #
/// # let mut world = World::new();
/// // Use `FilteredResourcesMutParamBuilder` to declare access to resources.
/// let system = (FilteredResourcesMutParamBuilder::new(|builder| {
///     builder.add_write::<B>().add_read::<C>().add_write::<D>();
/// }),)
///     .build_state(&mut world)
///     .build_system(resource_system);
///
/// world.init_resource::<A>();
/// world.init_resource::<C>();
/// world.init_resource::<D>();
///
/// fn resource_system(mut res: FilteredResourcesMut) {
///     // The resource exists, but we have no access, so we can't read it or write it.
///     assert!(res.get::<A>().is_err());
///     assert!(res.get_mut::<A>().is_err());
///     // The resource doesn't exist, so we can't read it or write it.
///     assert!(res.get::<B>().is_err());
///     assert!(res.get_mut::<B>().is_err());
///     // The resource exists and we have read access, so we can read it but not write it.
///     let c = res.get::<C>().unwrap();
///     assert!(res.get_mut::<C>().is_err());
///     // The resource exists and we have write access, so we can read it or write it.
///     let d = res.get::<D>().unwrap();
///     let d = res.get_mut::<D>().unwrap();
///     // The type parameter can be left out if it can be determined from use.
///     let c: Ref<C> = res.get().unwrap();
/// }
/// #
/// # world.run_system_once(system);
/// ```
///
/// This can be used alongside ordinary [`Res`](crate::system::ResMut) and [`ResMut`](crate::system::ResMut) parameters if they do not conflict.
///
/// ```
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # #[derive(Default, Resource)]
/// # struct B;
/// #
/// # #[derive(Default, Resource)]
/// # struct C;
/// #
/// # let mut world = World::new();
/// # world.init_resource::<A>();
/// # world.init_resource::<B>();
/// # world.init_resource::<C>();
/// #
/// let system = (
///     FilteredResourcesMutParamBuilder::new(|builder| {
///         builder.add_read::<A>().add_write::<B>();
///     }),
///     ParamBuilder,
///     ParamBuilder,
/// )
///     .build_state(&mut world)
///     .build_system(resource_system);
///
/// // Read access to A does not conflict with read access to A or write access to C.
/// // Write access to B does not conflict with access to A or C.
/// fn resource_system(mut filtered: FilteredResourcesMut, res_a: Res<A>, res_mut_c: ResMut<C>) {
///     let res_a_2: Ref<A> = filtered.get::<A>().unwrap();
///     let res_mut_b: Mut<B> = filtered.get_mut::<B>().unwrap();
/// }
/// #
/// # world.run_system_once(system);
/// ```
///
/// But it will conflict if it tries to read the same resource that another parameter writes,
/// or write the same resource that another parameter reads.
///
/// ```should_panic
/// # use bevy_ecs::{prelude::*, system::*};
/// #
/// # #[derive(Default, Resource)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.init_resource::<A>();
/// #
/// let system = (
///     FilteredResourcesMutParamBuilder::new(|builder| {
///         builder.add_write::<A>();
///     }),
///     ParamBuilder,
/// )
///     .build_state(&mut world)
///     .build_system(invalid_resource_system);
///
/// // Read access to A conflicts with write access to A.
/// fn invalid_resource_system(filtered: FilteredResourcesMut, res_a: Res<A>) { }
/// #
/// # world.run_system_once(system);
/// ```
pub struct FilteredResourcesMut<'w, 's> {
    world: UnsafeWorldCell<'w>,
    access: &'s Access,
    last_run: Tick,
    this_run: Tick,
}

impl<'w, 's> FilteredResourcesMut<'w, 's> {
    /// Creates a new [`FilteredResources`].
    /// # Safety
    /// It is the callers responsibility to ensure that nothing else may access the any resources in the `world` in a way that conflicts with `access`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        access: &'s Access,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            world,
            access,
            last_run,
            this_run,
        }
    }

    /// Gets read-only access to all of the resources this `FilteredResourcesMut` can access.
    pub fn as_readonly(&self) -> FilteredResources<'_, 's> {
        FilteredResources::from(self)
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut FilteredResourcesMut`, but you need `FilteredResourcesMut`.
    pub fn reborrow(&mut self) -> FilteredResourcesMut<'_, 's> {
        // SAFETY: We have exclusive access to this access for the duration of `'_`, so there cannot be anything else that conflicts.
        unsafe { Self::new(self.world, self.access, self.last_run, self.this_run) }
    }

    /// Returns a reference to the underlying [`Access`].
    pub fn access(&self) -> &Access {
        self.access
    }

    /// Returns `true` if the `FilteredResources` has read access to the given resource.
    /// Note that [`Self::get()`] may still return `Err` if the resource does not exist.
    pub fn has_read<R: Resource>(&self) -> bool {
        let component_id = self.world.components().resource_id::<R>();
        component_id.is_some_and(|component_id| self.access.has_resource_read(component_id))
    }

    /// Returns `true` if the `FilteredResources` has write access to the given resource.
    /// Note that [`Self::get_mut()`] may still return `Err` if the resource does not exist.
    pub fn has_write<R: Resource>(&self) -> bool {
        let component_id = self.world.components().resource_id::<R>();
        component_id.is_some_and(|component_id| self.access.has_resource_write(component_id))
    }

    /// Gets a reference to the resource of the given type if it exists and the `FilteredResources` has access to it.
    pub fn get<R: Resource>(&self) -> Result<Ref<'_, R>, ResourceFetchError> {
        self.as_readonly().get()
    }

    /// Gets a pointer to the resource with the given [`ComponentId`] if it exists and the `FilteredResources` has access to it.
    pub fn get_by_id(&self, component_id: ComponentId) -> Result<Ptr<'_>, ResourceFetchError> {
        self.as_readonly().get_by_id(component_id)
    }

    /// Gets a mutable reference to the resource of the given type if it exists and the `FilteredResources` has access to it.
    pub fn get_mut<R: Resource>(&mut self) -> Result<Mut<'_, R>, ResourceFetchError> {
        // SAFETY: We have exclusive access to the resources in `access` for `'_`, and we shorten the returned lifetime to that.
        unsafe { self.get_mut_unchecked() }
    }

    /// Gets a mutable pointer to the resource with the given [`ComponentId`] if it exists and the `FilteredResources` has access to it.
    pub fn get_mut_by_id(
        &mut self,
        component_id: ComponentId,
    ) -> Result<MutUntyped<'_>, ResourceFetchError> {
        // SAFETY: We have exclusive access to the resources in `access` for `'_`, and we shorten the returned lifetime to that.
        unsafe { self.get_mut_by_id_unchecked(component_id) }
    }

    /// Consumes self and gets mutable access to resource of the given type with the world `'w` lifetime if it exists and the `FilteredResources` has access to it.
    pub fn into_mut<R: Resource>(mut self) -> Result<Mut<'w, R>, ResourceFetchError> {
        // SAFETY: This consumes self, so we have exclusive access to the resources in `access` for the entirety of `'w`.
        unsafe { self.get_mut_unchecked() }
    }

    /// Consumes self and gets mutable access to resource with the given [`ComponentId`] with the world `'w` lifetime if it exists and the `FilteredResources` has access to it.
    pub fn into_mut_by_id(
        mut self,
        component_id: ComponentId,
    ) -> Result<MutUntyped<'w>, ResourceFetchError> {
        // SAFETY: This consumes self, so we have exclusive access to the resources in `access` for the entirety of `'w`.
        unsafe { self.get_mut_by_id_unchecked(component_id) }
    }

    /// Gets a mutable pointer to the resource of the given type if it exists and the `FilteredResources` has access to it.
    /// # Safety
    /// It is the callers responsibility to ensure that there are no conflicting borrows of anything in `access` for the duration of the returned value.
    unsafe fn get_mut_unchecked<R: Resource>(&mut self) -> Result<Mut<'w, R>, ResourceFetchError> {
        let component_id = self
            .world
            .components()
            .valid_resource_id::<R>()
            .ok_or(ResourceFetchError::NotRegistered)?;
        // SAFETY: THe caller ensures that there are no conflicting borrows.
        unsafe { self.get_mut_by_id_unchecked(component_id) }
            // SAFETY: The underlying type of the resource is `R`.
            .map(|ptr| unsafe { ptr.with_type::<R>() })
    }

    /// Gets a mutable pointer to the resource with the given [`ComponentId`] if it exists and the `FilteredResources` has access to it.
    /// # Safety
    /// It is the callers responsibility to ensure that there are no conflicting borrows of anything in `access` for the duration of the returned value.
    unsafe fn get_mut_by_id_unchecked(
        &mut self,
        component_id: ComponentId,
    ) -> Result<MutUntyped<'w>, ResourceFetchError> {
        if !self.access.has_resource_write(component_id) {
            return Err(ResourceFetchError::NoResourceAccess(component_id));
        }

        // SAFETY: We have read access to this resource
        let (value, ticks, caller) = unsafe { self.world.get_resource_with_ticks(component_id) }
            .ok_or(ResourceFetchError::DoesNotExist(component_id))?;

        Ok(MutUntyped {
            // SAFETY: We have exclusive access to the underlying storage.
            value: unsafe { value.assert_unique() },
            // SAFETY: We have exclusive access to the underlying storage.
            ticks: unsafe { TicksMut::from_tick_cells(ticks, self.last_run, self.this_run) },
            // SAFETY: We have exclusive access to the underlying storage.
            changed_by: unsafe { caller.map(|caller| caller.deref_mut()) },
        })
    }
}

impl<'w> From<&'w mut World> for FilteredResourcesMut<'w, 'static> {
    fn from(value: &'w mut World) -> Self {
        const WRITE_ALL_RESOURCES: &Access = {
            const ACCESS: Access = {
                let mut access = Access::new();
                access.write_all_resources();
                access
            };
            &ACCESS
        };

        let last_run = value.last_change_tick();
        let this_run = value.change_tick();
        // SAFETY: We have a mutable reference to the entire world, so nothing else can alias with mutable access to all resources.
        unsafe {
            Self::new(
                value.as_unsafe_world_cell_readonly(),
                WRITE_ALL_RESOURCES,
                last_run,
                this_run,
            )
        }
    }
}

/// Builder struct to define the access for a [`FilteredResources`].
///
/// This is passed to a callback in [`FilteredResourcesParamBuilder`](crate::system::FilteredResourcesParamBuilder).
pub struct FilteredResourcesBuilder<'w> {
    world: &'w mut World,
    access: Access,
}

impl<'w> FilteredResourcesBuilder<'w> {
    /// Creates a new builder with no access.
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            access: Access::new(),
        }
    }

    /// Returns a reference to the underlying [`Access`].
    pub fn access(&self) -> &Access {
        &self.access
    }

    /// Add accesses required to read all resources.
    pub fn add_read_all(&mut self) -> &mut Self {
        self.access.read_all_resources();
        self
    }

    /// Add accesses required to read the resource of the given type.
    pub fn add_read<R: Resource>(&mut self) -> &mut Self {
        let component_id = self.world.components_registrator().register_resource::<R>();
        self.add_read_by_id(component_id)
    }

    /// Add accesses required to read the resource with the given [`ComponentId`].
    pub fn add_read_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.access.add_resource_read(component_id);
        self
    }

    /// Create an [`Access`] that represents the accesses of the builder.
    pub fn build(self) -> Access {
        self.access
    }
}

/// Builder struct to define the access for a [`FilteredResourcesMut`].
///
/// This is passed to a callback in [`FilteredResourcesMutParamBuilder`](crate::system::FilteredResourcesMutParamBuilder).
pub struct FilteredResourcesMutBuilder<'w> {
    world: &'w mut World,
    access: Access,
}

impl<'w> FilteredResourcesMutBuilder<'w> {
    /// Creates a new builder with no access.
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            access: Access::new(),
        }
    }

    /// Returns a reference to the underlying [`Access`].
    pub fn access(&self) -> &Access {
        &self.access
    }

    /// Add accesses required to read all resources.
    pub fn add_read_all(&mut self) -> &mut Self {
        self.access.read_all_resources();
        self
    }

    /// Add accesses required to read the resource of the given type.
    pub fn add_read<R: Resource>(&mut self) -> &mut Self {
        let component_id = self.world.components_registrator().register_resource::<R>();
        self.add_read_by_id(component_id)
    }

    /// Add accesses required to read the resource with the given [`ComponentId`].
    pub fn add_read_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.access.add_resource_read(component_id);
        self
    }

    /// Add accesses required to get mutable access to all resources.
    pub fn add_write_all(&mut self) -> &mut Self {
        self.access.write_all_resources();
        self
    }

    /// Add accesses required to get mutable access to the resource of the given type.
    pub fn add_write<R: Resource>(&mut self) -> &mut Self {
        let component_id = self.world.components_registrator().register_resource::<R>();
        self.add_write_by_id(component_id)
    }

    /// Add accesses required to get mutable access to the resource with the given [`ComponentId`].
    pub fn add_write_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.access.add_resource_write(component_id);
        self
    }

    /// Create an [`Access`] that represents the accesses of the builder.
    pub fn build(self) -> Access {
        self.access
    }
}
