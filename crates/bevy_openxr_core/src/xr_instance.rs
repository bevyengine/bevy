use once_cell::sync::OnceCell;
use std::fmt;
use wgpu::wgpu_openxr::WGPUOpenXR;

use crate::{OpenXRStruct, XRDevice, XrOptions};

/// Used to transfer the at-app-beginning initializable openxr device for bevy
static mut XR_INSTANCE: OnceCell<XrInstance> = OnceCell::new();

pub struct XrInstance {
    wgpu_openxr: WGPUOpenXR,
    inner: openxr::Instance,
}

impl XrInstance {
    pub fn new(wgpu_openxr: WGPUOpenXR, instance: openxr::Instance) -> Self {
        Self {
            wgpu_openxr,
            inner: instance,
        }
    }

    pub(crate) fn into_device_with_options(self, options: XrOptions) -> XRDevice {
        let handles = self.wgpu_openxr.get_session_handles().unwrap();
        let xr_struct = OpenXRStruct::new(self.inner, handles, options);

        XRDevice::new(xr_struct)
    }
}

impl fmt::Debug for XrInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XrInstance[]")
    }
}

/// Set the openxr device from initialization code - will be later used by bevy
/// Should be called exactly once
pub fn set_xr_instance(instance: XrInstance) {
    unsafe { XR_INSTANCE.set(instance).unwrap() };
}

pub(crate) fn take_xr_instance() -> XrInstance {
    match unsafe { XR_INSTANCE.take() } {
        Some(instance) => instance,
        None => panic!("Must call set_xr_instance"),
    }
}
