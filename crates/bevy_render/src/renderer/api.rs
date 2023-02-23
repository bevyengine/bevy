use std::{path::Path, pin::Pin};

use bevy_window::ThreadLockedRawWindowHandleWrapper;
use futures_lite::Future;
use wgpu::{
    Adapter, CreateSurfaceError, Device, DeviceDescriptor, Instance, InstanceDescriptor, Queue,
    RequestAdapterOptions, RequestDeviceError, Surface,
};

/// This trait is intended to be used for hooking into the [`Instance`], [`Surface`], [`Adapter`]
/// and [`Device`] creation process of the [`RenderPlugin`](crate::RenderPlugin).
///
/// To do this, insert a [`RenderApi`](crate::RenderApi) resource before adding the
/// [`RenderPlugin`](crate::RenderPlugin).
pub trait Api: Send + Sync {
    /// Creates a new wgpu [`Instance`].
    ///
    /// Implement this if you need custom instance creation logic,
    /// e.g. you want to enable additional Vulkan instance extensions.
    ///
    /// For the default implementation, see [`Instance::new()`].
    fn new_instance(&self, instance_desc: InstanceDescriptor) -> Instance {
        Instance::new(instance_desc)
    }

    /// Creates a [`Surface`] from a raw window handle for a given [`Instance`].
    ///
    /// Implement this if you need custom creation logic.
    ///
    /// For the default implementation, see [`Instance::create_surface()`].
    ///
    /// # Safety
    ///
    /// - `raw_window_handle` must be a valid object to create a surface upon.
    /// - `raw_window_handle` must remain valid until after the returned [`Surface`] is
    ///   dropped.
    unsafe fn create_surface(
        &self,
        instance: &Instance,
        raw_window_handle: &ThreadLockedRawWindowHandleWrapper,
    ) -> Result<Surface, CreateSurfaceError> {
        instance.create_surface(raw_window_handle)
    }

    /// Retrieves an [`Adapter`] which matches the given [`RequestAdapterOptions`]
    /// for a given [`Instance`].
    ///
    /// Implement this if you have additional requirements on the adapter,
    /// e.g. you need to make sure that a Device supports a specific extension or feature.
    ///
    /// For the default implementation, see [`Instance::request_adapter()`].
    fn request_adapter(
        &self,
        instance: &Instance,
        options: &RequestAdapterOptions,
    ) -> RequestAdapterFuture {
        Box::pin(instance.request_adapter(options))
    }

    /// Requests a connection to a physical device, creating a logical [`Device`]
    /// for a given [`Adapter`].
    ///
    /// Implement this if you need custom device creation logic,
    /// e.g. you want to enable additional Vulkan device extensions.
    ///
    /// For the default implementation, see [`Adapter::request_device()`].
    fn request_device(
        &self,
        adapter: &Adapter,
        desc: &DeviceDescriptor,
        trace_path: Option<&Path>,
    ) -> RequestDeviceFuture {
        Box::pin(adapter.request_device(desc, trace_path))
    }
}

/// An implementation of [`Api`], using only default method implementations.
pub struct DefaultApi;

impl Api for DefaultApi {}

/// The [`Future`] returned by [`Api::request_adapter()`]
pub type RequestAdapterFuture = Pin<Box<dyn Future<Output = Option<Adapter>>>>;

/// The [`Future`] returned by [`Api::request_device()`]
pub type RequestDeviceFuture =
    Pin<Box<dyn Future<Output = Result<(Device, Queue), RequestDeviceError>>>>;
