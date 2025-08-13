#[doc(hidden)]
pub use crate::{
    app::prelude::*, ecs::prelude::*, input::prelude::*, math::prelude::*, platform::prelude::*,
    reflect::prelude::*, time::prelude::*, transform::prelude::*, utils::prelude::*,
    DefaultPlugins, MinimalPlugins,
};

#[doc(hidden)]
#[cfg(feature = "bevy_log")]
pub use crate::log::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_window")]
pub use crate::window::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_image")]
pub use crate::image::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_mesh")]
pub use crate::mesh::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_light")]
pub use crate::light::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_camera")]
pub use crate::camera::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_shader")]
pub use crate::shader::prelude::*;

pub use bevy_derive::{bevy_main, Deref, DerefMut};

#[doc(hidden)]
#[cfg(feature = "bevy_asset")]
pub use crate::asset::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_audio")]
pub use crate::audio::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_animation")]
pub use crate::animation::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_color")]
pub use crate::color::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_pbr")]
pub use crate::pbr::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_render")]
pub use crate::render::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_scene")]
pub use crate::scene::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_sprite")]
pub use crate::sprite::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_text")]
pub use crate::text::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_ui")]
pub use crate::ui::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_ui_render")]
pub use crate::ui_render::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_gizmos")]
pub use crate::gizmos::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_gilrs")]
pub use crate::gilrs::*;

#[doc(hidden)]
#[cfg(feature = "bevy_state")]
pub use crate::state::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_gltf")]
pub use crate::gltf::prelude::*;

#[doc(hidden)]
#[cfg(feature = "bevy_picking")]
pub use crate::picking::prelude::*;
