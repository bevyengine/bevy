pub use crate::{
    app::prelude::*, asset::prelude::*, core::prelude::*, ecs::prelude::*, input::prelude::*,
    log::prelude::*, math::prelude::*, reflect::prelude::*, scene::prelude::*,
    transform::prelude::*, window::prelude::*, DefaultPlugins, MinimalPlugins,
};

pub use bevy_derive::bevy_main;

#[cfg(feature = "bevy_audio")]
pub use crate::audio::prelude::*;

#[cfg(feature = "bevy_pbr")]
pub use crate::pbr::prelude::*;

#[cfg(feature = "bevy_render")]
pub use crate::render::prelude::*;

#[cfg(feature = "bevy_sprite")]
pub use crate::sprite::prelude::*;

#[cfg(feature = "bevy_text")]
pub use crate::text::prelude::*;

#[cfg(feature = "bevy_ui")]
pub use crate::ui::prelude::*;

#[cfg(feature = "bevy_dynamic_plugin")]
pub use crate::dynamic_plugin::*;

#[cfg(feature = "bevy_gilrs")]
pub use crate::gilrs::*;
