use alloc::sync::Arc;

use bevy_asset::{Assets, Handle};
use bevy_ecs::system::{Res, ResMut, SystemParam};
use cosmic_text::fontdb::FaceInfo;

use crate::{CosmicFontSystem, Font, TextPipeline};

/// Provides a method for finding [fonts](`Font`) based on their [`FaceInfo`].
///
/// Note that this is most useful with the `system_font` feature, which exposes
/// fonts installed on the end-users device. Without this feature, the only fonts
/// available are ones explicitly added within Bevy anyway.
#[derive(SystemParam)]
pub struct FontLibrary<'w> {
    text_pipeline: ResMut<'w, TextPipeline>,
    font_system: Res<'w, CosmicFontSystem>,
    font_assets: ResMut<'w, Assets<Font>>,
}

impl FontLibrary<'_> {
    /// Find a [`Font`] based on the provided criteria.
    /// You are given access to the font's [`FaceInfo`] to aid with selection.
    pub fn find(&mut self, mut f: impl FnMut(&FaceInfo) -> bool) -> Option<Handle<Font>> {
        self.font_system.db().faces().find_map(|face_info| {
            if !f(face_info) {
                return None;
            };

            let face_id = face_info.id;

            // TODO: If multiple families are present, should all be added?
            let family_name = Arc::from(face_info.families[0].0.as_str());

            let font = Font {
                // TODO: The binary data isn't accessible (or required) for fonts loaded
                // from the system. Perhaps an enum should be used to indicate this
                // is deliberately empty, but still represents a valid font.
                data: Arc::default(),
            };

            let font = self.font_assets.add(font);

            self.text_pipeline
                .register_font(font.id(), face_id, family_name);

            Some(font)
        })
    }
}
