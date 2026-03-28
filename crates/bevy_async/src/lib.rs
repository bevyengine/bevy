#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

mod async_bridge;
mod ecs_access;
mod plugin;
mod system_state_store;
mod wake_signal;

pub use crate::async_bridge::async_world_sync_point;
pub use crate::ecs_access::{AsyncSystemState, EcsAccessError};
pub use crate::plugin::{AsyncPlugin, AsyncWorld};

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        async_world_sync_point, AsyncPlugin, AsyncSystemState, AsyncWorld, EcsAccessError,
    };
}
