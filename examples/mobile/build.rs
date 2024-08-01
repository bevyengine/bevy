#![allow(missing_docs)]

use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "android" {
        android();
    }
}

fn android() {
    println!("cargo:rustc-link-lib=c++_shared");

    if let Ok(output_path) = env::var("CARGO_NDK_OUTPUT_PATH") {
        let sysroot_libs_path = PathBuf::from(env::var_os("CARGO_NDK_SYSROOT_LIBS_PATH").unwrap());
        let lib_path = sysroot_libs_path.join("libc++_shared.so");
        let dest_path = Path::new(&output_path)
            .join(env::var("CARGO_NDK_ANDROID_TARGET").unwrap())
            .join("libc++_shared.so");
        println!("cargo:rerun-if-changed={:?}", &*dest_path); // use this to force libc++_shared.so to get copied to jniLibs even if project hasn't changed
        if let Err(e) = std::fs::create_dir_all(dest_path.clone().parent().unwrap()) {
            panic!("Unable to create output dir: {:?}", e);
        }
        if let Err(e) = std::fs::copy(lib_path, dest_path) {
            panic!("Unable to copy libc++_shared.so: {}", e);
        }
    } else {
        panic!("CARGO_NDK_OUTPUT_PATH not set.");
    }
}
