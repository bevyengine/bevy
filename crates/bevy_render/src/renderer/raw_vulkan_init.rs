use alloc::sync::Arc;
use bevy_ecs::resource::Resource;
use bevy_platform::collections::HashSet;
use core::any::{Any, TypeId};
use thiserror::Error;
use wgpu::{
    hal::api::Vulkan, Adapter, Device, DeviceDescriptor, Instance, InstanceDescriptor, Queue,
};

/// When the `raw_vulkan_init` feature is enabled, these settings will be used to configure the raw vulkan instance.
#[derive(Resource, Default, Clone)]
pub struct RawVulkanInitSettings {
    // SAFETY: this must remain private to ensure that registering callbacks is unsafe
    create_instance_callbacks: Vec<
        Arc<
            dyn Fn(
                    &mut wgpu::hal::vulkan::CreateInstanceCallbackArgs,
                    &mut AdditionalVulkanFeatures,
                ) + Send
                + Sync,
        >,
    >,
    // SAFETY: this must remain private to ensure that registering callbacks is unsafe
    create_device_callbacks: Vec<
        Arc<
            dyn Fn(
                    &mut wgpu::hal::vulkan::CreateDeviceCallbackArgs,
                    &wgpu::hal::vulkan::Adapter,
                    &mut AdditionalVulkanFeatures,
                ) + Send
                + Sync,
        >,
    >,
}

impl RawVulkanInitSettings {
    /// Adds a new Vulkan create instance callback. See [`wgpu::hal::vulkan::Instance::init_with_callback`] for details.
    ///
    /// # Safety
    /// - Callback must not remove features.
    /// - Callback must not change anything to what the instance does not support.
    pub unsafe fn add_create_instance_callback(
        &mut self,
        callback: impl Fn(&mut wgpu::hal::vulkan::CreateInstanceCallbackArgs, &mut AdditionalVulkanFeatures)
            + Send
            + Sync
            + 'static,
    ) {
        self.create_instance_callbacks.push(Arc::new(callback));
    }

    /// Adds a new Vulkan create device callback. See [`wgpu::hal::vulkan::Adapter::open_with_callback`] for details.
    ///
    /// # Safety
    /// - Callback must not remove features.
    /// - Callback must not change anything to what the device does not support.
    pub unsafe fn add_create_device_callback(
        &mut self,
        callback: impl Fn(
                &mut wgpu::hal::vulkan::CreateDeviceCallbackArgs,
                &wgpu::hal::vulkan::Adapter,
                &mut AdditionalVulkanFeatures,
            ) + Send
            + Sync
            + 'static,
    ) {
        self.create_device_callbacks.push(Arc::new(callback));
    }
}

pub(crate) fn create_raw_vulkan_instance(
    instance_descriptor: &InstanceDescriptor,
    settings: &RawVulkanInitSettings,
    additional_features: &mut AdditionalVulkanFeatures,
) -> Instance {
    // SAFETY: Registering callbacks is unsafe. Callback authors promise not to remove features
    // or change the instance to something it does not support
    unsafe {
        wgpu::hal::vulkan::Instance::init_with_callback(
            &wgpu::hal::InstanceDescriptor {
                name: "wgpu",
                flags: instance_descriptor.flags,
                memory_budget_thresholds: instance_descriptor.memory_budget_thresholds,
                backend_options: instance_descriptor.backend_options.clone(),
            },
            Some(Box::new(|mut args| {
                for callback in &settings.create_instance_callbacks {
                    (callback)(&mut args, additional_features);
                }
            })),
        )
        .map(|raw_instance| Instance::from_hal::<Vulkan>(raw_instance))
        .unwrap_or_else(|_| Instance::new(instance_descriptor))
    }
}

pub(crate) async fn create_raw_device(
    adapter: &Adapter,
    device_descriptor: &DeviceDescriptor<'_>,
    settings: &RawVulkanInitSettings,
    additional_features: &mut AdditionalVulkanFeatures,
) -> Result<(Device, Queue), CreateRawVulkanDeviceError> {
    // SAFETY: Registering callbacks is unsafe. Callback authors promise not to remove features
    // or change the adapter to something it does not support
    unsafe {
        let Some(raw_adapter) = adapter.as_hal::<Vulkan>() else {
            return Ok(adapter.request_device(device_descriptor).await?);
        };
        let open_device = raw_adapter.open_with_callback(
            device_descriptor.required_features,
            &device_descriptor.memory_hints,
            Some(Box::new(|mut args| {
                for callback in &settings.create_device_callbacks {
                    (callback)(&mut args, &raw_adapter, additional_features);
                }
            })),
        )?;

        Ok(adapter.create_device_from_hal::<Vulkan>(open_device, device_descriptor)?)
    }
}

#[derive(Error, Debug)]
pub(crate) enum CreateRawVulkanDeviceError {
    #[error(transparent)]
    RequestDeviceError(#[from] wgpu::RequestDeviceError),
    #[error(transparent)]
    DeviceError(#[from] wgpu::hal::DeviceError),
}

/// A list of additional Vulkan features that are supported by the current wgpu instance / adapter. This is populated
/// by callbacks defined in [`RawVulkanInitSettings`]
#[derive(Resource, Default, Clone)]
pub struct AdditionalVulkanFeatures(HashSet<TypeId>);

impl AdditionalVulkanFeatures {
    pub fn insert<T: Any>(&mut self) {
        self.0.insert(TypeId::of::<T>());
    }

    pub fn has<T: Any>(&self) -> bool {
        self.0.contains(&TypeId::of::<T>())
    }

    pub fn remove<T: Any>(&mut self) {
        self.0.remove(&TypeId::of::<T>());
    }
}
