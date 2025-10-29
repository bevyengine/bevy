use crate::context::FontCx;
use crate::TextFont;
use bevy_asset::Asset;
use bevy_asset::AssetEvent;
use bevy_asset::Assets;
use bevy_ecs::change_detection::DetectChangesMut;
use bevy_ecs::message::MessageReader;
use bevy_ecs::system::Query;
use bevy_ecs::system::ResMut;
use bevy_reflect::TypePath;
use bevy_utils::default;
use parley::fontique::Blob;
use parley::fontique::FontInfoOverride;

/// An [`Asset`] that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
///
/// # A note on fonts
///
/// `Font` may differ from the everyday notion of what a "font" is.
/// A font *face* (e.g. Fira Sans Semibold Italic) is part of a font *family* (e.g. Fira Sans),
/// and is distinguished from other font faces in the same family
/// by its style (e.g. italic), its weight (e.g. bold) and its stretch (e.g. condensed).
///
/// Bevy currently loads a single font face as a single `Font` asset.
#[derive(Debug, TypePath, Clone, Asset)]
pub struct Font {
    pub blob: Blob<u8>,
    pub family_name: String,
}

impl Font {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(font_data: Vec<u8>, family_name: String) -> Font {
        Font {
            blob: Blob::from(font_data),
            family_name,
        }
    }
}

pub fn register_font_assets_system(
    mut cx: ResMut<FontCx>,
    mut fonts: ResMut<Assets<Font>>,
    mut events: MessageReader<AssetEvent<Font>>,
    mut text_font_query: Query<&mut TextFont>,
) {
    for event in events.read() {
        match event {
            AssetEvent::Added { id } => {
                if let Some(font) = fonts.get_mut(*id) {
                    cx.collection.register_fonts(
                        font.blob.clone(),
                        Some(FontInfoOverride {
                            family_name: Some(font.family_name.as_str()),
                            ..default()
                        }),
                    );
                    for mut font in text_font_query.iter_mut() {
                        if font.font.id() == *id {
                            font.set_changed();
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
