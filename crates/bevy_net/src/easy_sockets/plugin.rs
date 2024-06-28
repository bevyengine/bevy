use bevy_internal::app::App;
use bevy_internal::core::TaskPoolPlugin;
use bevy_internal::prelude::Plugin;


pub struct SocketManagerPlugin {}

impl Plugin for SocketManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TaskPoolPlugin)
        todo!()
    }
}