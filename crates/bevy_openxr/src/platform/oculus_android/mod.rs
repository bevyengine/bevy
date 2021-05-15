use std::{ffi, ptr};

use openxr::{Entry, ExtensionSet, Instance};

use super::OpenXRInstance;
use crate::error::Error;

impl OpenXRInstance for openxr::Entry {
    fn load_bevy_openxr() -> Result<openxr::Entry, Error> {
        // Dynamic loading of the library
        // Expects lib/[arm64-v8a, ...]/libopenxr_loader.so to be present
        // libopenxr_loader.so is provided by Oculus mobile SDK
        // https://developer.oculus.com/downloads/package/oculus-openxr-mobile-sdk/
        let entry = Entry::load()?;

        // FIXME SAFETY need to send nullptr (as per OpenXR docs), is this safe enough?
        let instance: openxr::sys::Instance = unsafe { std::mem::zeroed() };

        // Get address pointer to xrInitializeLoaderKHR through xrGetInstanceProcAddress
        let loader_init_khr = unsafe { openxr::raw::LoaderInitKHR::load(&entry, instance) }?;

        // construct XrLoaderInitInfoAndroidKHR
        // https://developer.oculus.com/downloads/package/oculus-openxr-mobile-sdk/
        let (application_vm, application_context) = get_android_vm_and_jni_context()?;
        let android_khr = openxr::sys::LoaderInitInfoAndroidKHR {
            ty: openxr::sys::StructureType::LOADER_INIT_INFO_ANDROID_KHR,
            next: ptr::null(),
            application_vm,
            application_context,
        };

        // call xrInitializeLoaderKHR with the Android info
        // must be called, otherwise loader library throws error "xrInitializeLoaderOCULUS"
        let ret = unsafe {
            (loader_init_khr.initialize_loader)(
                &android_khr as *const _ as *const openxr::sys::LoaderInitInfoBaseHeaderKHR,
            )
        };

        // Handle result
        if ret == openxr::sys::Result::SUCCESS {
            Ok(entry)
        } else {
            Err(Error::XR(ret))
        }
    }

    fn instantiate(&mut self, extensions: &ExtensionSet) -> Result<Instance, Error> {
        let (application_vm, application_context) = get_android_vm_and_jni_context()?;

        let android_info = openxr::sys::InstanceCreateInfoAndroidKHR {
            ty: openxr::sys::StructureType::INSTANCE_CREATE_INFO_ANDROID_KHR,
            next: ptr::null(),
            application_vm,
            application_activity: application_context,
        };

        /*
        let other_extensions = extensions.other
            .iter()
            .map(|x| if x.contains("XR_FB_display_refresh_rate") { Some(x) } else { None })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();
             */
        // FIXME submit extensions, and check at create_instance time if supported extension set has it...?

        let other_extensions = Vec::new();

        let xr_instance = self.create_instance(
            &openxr::ApplicationInfo {
                application_name: "hello openxr",
                engine_name: "bevy",
                application_version: 1, // FIXME allow user to submit application version?
                engine_version: 1,      // FIXME pull bevy version from somewhere?
            },
            &extensions,
            Some(other_extensions),
            Some(&android_info as *const _ as *const std::os::raw::c_void),
            &[],
        )?;

        Ok(xr_instance)
    }
}

fn get_android_vm_and_jni_context() -> Result<(*mut ffi::c_void, *mut ffi::c_void), Error> {
    // JNI & Android activity are needed by Oculus runtime
    // modified from
    // https://github.com/rust-windowing/android-ndk-rs/blob/master/ndk-examples/examples/jni_audio.rs
    let native_activity = ndk_glue::native_activity();
    let vm_ptr = native_activity.vm();
    let vm = unsafe { jni::JavaVM::from_raw(vm_ptr) }?;
    let vm_pointer = vm.get_java_vm_pointer();
    let application_vm = vm_pointer as *mut ffi::c_void;
    let application_context = native_activity.activity() as *mut _ as *mut ffi::c_void;

    Ok((application_vm, application_context))
}
