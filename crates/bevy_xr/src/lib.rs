pub mod interaction;
pub mod presentation;

use bevy_app::{AppBuilder, Plugin};
use interaction::{VibrationEvent, XrAxes, XrButtons, XrProfiles};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrMode {
    ImmersiveVR,
    ImmersiveAR,
    InlineVR,
    InlineAR,
}

#[derive(Default)]
pub struct XrPlugin;

impl Plugin for XrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<XrButtons>()
            .init_resource::<XrAxes>()
            .add_event::<VibrationEvent>()
            .init_resource::<XrProfiles>();
    }
}
