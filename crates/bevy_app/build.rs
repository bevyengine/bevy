//! Build script containing cfg aliases

fn main() {
    // Collecting information that will be used to decide which cfg aliases
    // will be exported.
    // Using `cfg!(target_arch = "wasm32")` and such does not work because
    // `build.rs` uses the host archtecture and other information, not the target's.
    // Cargo then defines environment variables for the information of the target
    let windows = std::env::var("CARGO_CFG_WINDOWS").is_ok();
    let unix = std::env::var("CARGO_CFG_UNIX").is_ok();

    let is_wasm32 = target_archtecture("wasm32");

    let feature_bevy_tasks = feature_present("bevy_tasks");
    let feature_std = feature_present("std");

    // Prevent warnings on uses of `#[cfg]`
    let cfgs = ["can_run_tasks", "std_windows_or_unix"];
    println!("cargo::rustc-check-cfg=cfg({})", cfgs.join(","));

    // Defining cfg aliases
    if !is_wasm32 && feature_bevy_tasks {
        println!("cargo::rustc-cfg=can_run_tasks");
    }
    if (windows || unix) && feature_std {
        println!("cargo::rustc-cfg=std_windows_or_unix");
    }
}

fn feature_present(feature: &str) -> bool {
    // This is very naive and may fail if there is `=`, `\0`, or unicode characters.
    let feature_name = format!("CARGO_FEATURE_{}", feature.to_uppercase().replace("-", "_"));
    std::env::var(feature_name.as_str())
        .ok()
        .filter(|feature| feature == "1")
        .is_some()
}

fn target_archtecture(target: &str) -> bool {
    std::env::var("CARGO_CFG_TARGET_ARCH")
        .ok()
        .filter(|arch| arch == target)
        .is_some()
}
