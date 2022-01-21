//! Audio support for the game engine Bevy
//!
//! ```
//! # use bevy_ecs::{system::Res, event::EventWriter};
//! # use bevy_audio::{Audio, AudioPlugin, PlayEvent};
//! # use bevy_asset::{AssetPlugin, AssetServer};
//! # use bevy_app::{App, AppExit};
//! # use bevy_internal::MinimalPlugins;
//! fn main() {
//!    App::new()
//!         .add_plugins(MinimalPlugins)
//!         .add_plugin(AssetPlugin)
//!         .add_plugin(AudioPlugin)
//! #       .add_system(stop)
//!         .add_startup_system(play_background_audio)
//!         .add_startup_system(loop_play_background_audio)
//!         .run();
//! }
//!
//! fn play_background_audio(asset_server: Res<AssetServer>, audio: Res<Audio>) {
//!     audio.play(asset_server.load("background_audio.ogg"));
//! }
//!
//!
//! fn loop_play_background_audio(asset_server: Res<AssetServer>, mut ew : EventWriter<PlayEvent>) {
//!     let sound = asset_server.load("background_audio.ogg");
//!     ew.send(PlayEvent::Loop(true));
//!     ew.send(PlayEvent::Append(sound));
//!
//!     let first = asset_server.load("start.ogg");
//!     ew.send(PlayEvent::Once(first));
//! }
//!
//! # fn stop(mut events: EventWriter<AppExit>) {
//! #     events.send(AppExit)
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod asset_player;
mod audio;
mod audio_output;
mod audio_source;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{AssetPlayer, Audio, AudioOutput, AudioSource, Decodable, PlayEvent};
}

pub use asset_player::*;
pub use audio::*;
pub use audio_output::*;
pub use audio_source::*;

use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use bevy_ecs::schedule::SystemSet;
use bevy_ecs::system::IntoExclusiveSystem;

/// Adds support for audio playback to a Bevy Application
///
/// Use the [`Audio`] resource to play audio.
#[derive(Default)]
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<AudioOutput<AudioSource>>()
            .init_non_send_resource::<AssetPlayer>()
            .add_event::<PlayEvent<AudioSource>>()
            .add_asset::<AudioSource>()
            .init_resource::<Audio<AudioSource>>()
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::new()
                    .with_system(play_queued_audio_system::<AudioSource>.exclusive_system())
                    .with_system(play_assets_system::<AudioSource>.exclusive_system()),
            );

        #[cfg(any(feature = "mp3", feature = "flac", feature = "wav", feature = "vorbis"))]
        app.init_asset_loader::<AudioLoader>();
    }
}
