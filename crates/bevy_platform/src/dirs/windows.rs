extern crate windows_sys as windows;
use alloc::slice;
use core::ffi::c_void;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use windows::Win32::UI::Shell;

/// Returns the path to the directory used for application settings.
// From https://github.com/dirs-dev/dirs-sys-rs/blob/main/src/lib.rs
#[expect(unsafe_code, reason = "Uses unsafe Windows API functions")]
fn known_folder(folder_id: windows::core::GUID) -> Option<PathBuf> {
    // SAFETY: SHGetKnownFolderPath allocates path_ptr which must be freed with CoTaskMemFree.
    unsafe {
        let mut path_ptr: windows::core::PWSTR = core::ptr::null_mut();
        let result =
            Shell::SHGetKnownFolderPath(&folder_id, 0, core::ptr::null_mut(), &mut path_ptr);
        if result == 0 {
            let len = windows::Win32::Globalization::lstrlenW(path_ptr) as usize;
            let path = slice::from_raw_parts(path_ptr, len);
            let ostr: OsString = OsStringExt::from_wide(path);
            windows::Win32::System::Com::CoTaskMemFree(path_ptr as *const c_void);
            Some(PathBuf::from(ostr))
        } else {
            windows::Win32::System::Com::CoTaskMemFree(path_ptr as *const c_void);
            None
        }
    }
}

/// Returns the path to the directory used for application settings.
pub fn preferences_dir() -> Option<PathBuf> {
    known_folder(Shell::FOLDERID_LocalAppData)
}
