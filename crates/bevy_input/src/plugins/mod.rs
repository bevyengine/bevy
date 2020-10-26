pub mod gamepadplugin;
pub mod inputplugin;
pub mod keyboardplugin;
pub mod mouseplugin;
pub mod touchplugin;

pub mod prelude {
    pub use super::{
        gamepadplugin::GamepadInputPlugin, inputplugin::InputPlugin,
        keyboardplugin::KeyboardInputPlugin, mouseplugin::MouseInputPlugin,
        touchplugin::TouchInputPlugin,
    };
}
