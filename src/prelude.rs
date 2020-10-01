pub use crate::{
    app::prelude::*, asset::prelude::*, core::prelude::*, ecs::prelude::*, input::prelude::*,
    math::prelude::*, property::prelude::*, scene::prelude::*, transform::prelude::*,
    type_registry::RegisterType, window::prelude::*, AddDefaultPlugins,
};

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
