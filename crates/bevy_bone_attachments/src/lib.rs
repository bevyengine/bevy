//! Bone attachments are used to connect a model to another

#![no_std]

pub mod relationship;
#[cfg(feature = "bevy_scene")]
pub mod scene;

extern crate alloc;

use bevy_app::Plugin;

/// Most frequently used objects of [`bevy_bone_attachments`](self) for easy access
pub mod prelude {
    pub use super::{
        relationship::{AttachedTo, AttachingModels},
        BoneAttachmentsPlugin,
    };
}

#[derive(Default)]
/// Plugin that setups bone attachments
pub struct BoneAttachmentsPlugin;

impl Plugin for BoneAttachmentsPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.register_type::<relationship::AttachingModels>()
            .register_type::<relationship::AttachedTo>();
    }
}
