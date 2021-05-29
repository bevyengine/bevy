use ash::vk::{self, Handle};
use bevy_xr::presentation::XrGraphicsContext;
use openxr as xr;
use std::{error::Error, ffi::CString, sync::Arc};
use wgpu_hal as hal;
#[cfg(windows)]
use winapi::um::d3d11::ID3D11Device;

#[derive(Clone)]
pub enum GraphicsContextHandles {
    Vulkan {
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        queue_family_index: u32,
        queue_index: u32,
    },
    #[cfg(windows)]
    D3D11 { device: *const ID3D11Device },
}

#[derive(Debug, thiserror::Error)]
#[error("Error creating HAL adapter")]
pub struct AdapterError;

pub fn create_graphics_context(
    instance: &xr::Instance,
    system: xr::SystemId,
) -> Result<(GraphicsContextHandles, XrGraphicsContext), Box<dyn Error>> {
    let device_descriptor = wgpu::DeviceDescriptor::default();

    if instance.exts().khr_vulkan_enable2.is_some() {
        let vk_entry = unsafe { ash::Entry::new().map_err(Box::new)? };

        // Vulkan 1.0 constrained by Oculus Go support.
        // todo: multiview support will require Vulkan 1.1 or specific extensions
        let vk_version = vk::make_api_version(1, 0, 0, 0);

        // todo: check requirements
        let _requirements = instance
            .graphics_requirements::<xr::Vulkan>(system)
            .unwrap();

        let vk_app_info = vk::ApplicationInfo::builder()
            .application_version(0)
            .engine_version(0)
            .api_version(vk_version);

        let mut flags = hal::InstanceFlags::empty();
        if cfg!(debug_assertions) {
            flags |= hal::InstanceFlags::VALIDATION;
            flags |= hal::InstanceFlags::DEBUG;
        }

        let instance_extensions = <hal::api::Vulkan as hal::Api>::Instance::required_extensions(
            &vk_entry, vk_version, flags,
        )
        .map_err(Box::new)?;
        let instance_extensions_ptrs = instance_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let vk_instance = unsafe {
            let vk_instance = instance
                .create_vulkan_instance(
                    system,
                    std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                    &vk::InstanceCreateInfo::builder()
                        .application_info(&vk_app_info)
                        .enabled_extension_names(&instance_extensions_ptrs)
                        as *const _ as *const _,
                )
                .map_err(Box::new)?
                .map_err(|e| Box::new(vk::Result::from_raw(e)))?;

            ash::Instance::load(
                vk_entry.static_fn(),
                vk::Instance::from_raw(vk_instance as _),
            )
        };
        let hal_instance = unsafe {
            <hal::api::Vulkan as hal::Api>::Instance::from_raw(
                vk_entry.clone(),
                vk_instance.clone(),
                vk_version,
                instance_extensions,
                flags,
                Box::new(instance.clone()),
            )
            .map_err(Box::new)?
        };

        let vk_physical_device = vk::PhysicalDevice::from_raw(
            instance
                .vulkan_graphics_device(system, vk_instance.handle().as_raw() as _)
                .map_err(Box::new)? as _,
        );
        let hal_exposed_adapter = hal_instance
            .expose_adapter(vk_physical_device)
            .ok_or_else(|| Box::new(AdapterError))?;

        let queue_family_index = unsafe {
            vk_instance
                .get_physical_device_queue_family_properties(vk_physical_device)
                .into_iter()
                .enumerate()
                .find_map(|(queue_family_index, info)| {
                    if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        Some(queue_family_index as u32)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let queue_index = 0;

        let device_extensions = hal_exposed_adapter
            .adapter
            .required_device_extensions(device_descriptor.features);
        let device_extensions_ptrs = device_extensions
            .iter()
            .map(|x| x.as_ptr())
            .collect::<Vec<_>>();

        let mut physical_features = hal_exposed_adapter
            .adapter
            .physical_device_features(&device_extensions, device_descriptor.features);

        let family_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])
            .build();
        let family_infos = [family_info];

        let vk_device = {
            let info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(&family_infos)
                .enabled_extension_names(&device_extensions_ptrs);
            let info = physical_features.add_to_device_create_builder(info).build();

            unsafe {
                let vk_device = instance
                    .create_vulkan_device(
                        system,
                        std::mem::transmute(vk_entry.static_fn().get_instance_proc_addr),
                        vk_physical_device.as_raw() as _,
                        &info as *const _ as *const _,
                    )
                    .map_err(Box::new)?
                    .map_err(|e| Box::new(vk::Result::from_raw(e)))?;

                ash::Device::load(vk_instance.fp_v1_0(), vk::Device::from_raw(vk_device as _))
            }
        };
        let hal_device = unsafe {
            hal_exposed_adapter
                .adapter
                .device_from_raw(
                    vk_device.clone(),
                    &device_extensions,
                    queue_family_index,
                    queue_index,
                )
                .map_err(Box::new)?
        };

        // let wgpu_instance = unsafe { wgpu::Instance::from_hal::<hal::api::Vulkan>(hal_instance) };
        // let wgpu_adapter = unsafe { wgpu_instance.adapter_from_hal(hal_exposed_adapter) };
        // let (wgpu_device, wgpu_queue) = unsafe {
        //     wgpu_adapter
        //         .device_from_hal(hal_device, &device_descriptor, None)
        //         .map_err(Box::new)?
        // };

        Ok((
            GraphicsContextHandles::Vulkan {
                instance: vk_instance,
                physical_device: vk_physical_device,
                device: vk_device,
                queue_family_index,
                queue_index,
            },
            todo!(),
            // XrGraphicsContext {
            //     instance: wgpu_instance,
            //     device: Arc::new(wgpu_device),
            //     queue: wgpu_queue,
            // },
        ))
    } else {
        #[cfg(windows)]
        if instance.exts().khr_d3d11_enable {
            todo!()
        }

        Err(Box::new(xr::sys::Result::ERROR_EXTENSION_NOT_PRESENT))
    }
}
