#[macro_use] extern crate log;
extern crate android_logger;

use log::Level;
use android_logger::Config;

include!("../3d/3d_scene.rs");

#[cfg(target_os = "android")]
ndk_glue::ndk_glue!(android_main);

#[cfg(target_os = "android")]
fn android_main() {
    android_logger::init_once(Config::default().with_min_level(Level::Trace));
    main();
}