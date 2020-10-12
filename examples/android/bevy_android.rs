include!("../3d/3d_scene.rs");

#[cfg_attr(target_os = "android", ndk_glue::main(logger(level = "trace", tag = "bevy_android"), backtrace = "full"))]
fn android_main() {
    main();
}