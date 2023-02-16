fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    // Passes the optimization level on to the crate.
    // Used by bevy_app to emit a warning if bevy_ecs isn't optimized.
    println!(
        "cargo:rustc-env=OPT_LEVEL={}",
        std::env::var("OPT_LEVEL").unwrap()
    );
}
