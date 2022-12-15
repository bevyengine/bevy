use bevy_ecs::system::Resource;

pub use ndk::asset::AssetManager;
pub use winit::platform::android::activity::*;

/// Resource containing AndroidApp
pub struct AndroidActivityApp {
    pub android_app: AndroidApp,
}

impl Resource for AndroidActivityApp {}
