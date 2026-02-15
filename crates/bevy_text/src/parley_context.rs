use crate::TextError;
use crate::{FontSmoothing, FontSource};
use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::resource::Resource;
use parley::LayoutContext;
use parley::{FontContext, GenericFamily};
use swash::scale::ScaleContext;

/// A font database and cache, used for font family resolution and text layout.
///
/// This resource is a wrapper around [`parley::FontContext`].
#[derive(Resource, Default, Deref, DerefMut)]
pub struct FontCx(pub FontContext);

impl FontCx {
    /// Get the family name associated with a [`FontSource`].
    ///
    /// If the `FontSource` is a `Handle`, returns `None`. The family name can be found by using the handle to look
    /// up the `Font` asset instead.
    pub fn get_family<'a>(&'a mut self, source: &'a FontSource) -> Option<&'a str> {
        let generic_family = match source {
            FontSource::Handle(_) => return None,
            FontSource::Family(family) => return Some(family.as_str()),
            FontSource::Serif => GenericFamily::Serif,
            FontSource::SansSerif => GenericFamily::SansSerif,
            FontSource::Cursive => GenericFamily::Cursive,
            FontSource::Fantasy => GenericFamily::Fantasy,
            FontSource::Monospace => GenericFamily::Monospace,
            FontSource::SystemUi => GenericFamily::SystemUi,
            FontSource::UiSerif => GenericFamily::UiSerif,
            FontSource::UiSansSerif => GenericFamily::UiSansSerif,
            FontSource::UiMonospace => GenericFamily::UiMonospace,
            FontSource::UiRounded => GenericFamily::UiRounded,
            FontSource::Emoji => GenericFamily::Emoji,
            FontSource::Math => GenericFamily::Math,
            FontSource::FangSong => GenericFamily::FangSong,
        };

        let family_id = self.0.collection.generic_families(generic_family).next();
        family_id.and_then(|id| self.0.collection.family_name(id))
    }

    /// Sets the fallback font for a given generic family.
    ///
    /// In most cases, these methods do not need to called manually,
    /// as [`parley::fontique`] will automatically select appropriate default fonts based based on available system fonts.
    ///
    /// Note that the `parley/system` feature must be enabled to allow automatic system font discovery.
    ///
    /// These methods will return an error if the provided family name does not already exist in the font collection.
    pub fn set_generic_family(
        &mut self,
        generic: GenericFamily,
        family_name: &str,
    ) -> Result<(), TextError> {
        self.collection
            .family_id(family_name)
            .ok_or(TextError::NoSuchFontFamily(family_name.to_string()))
            .map(|id| {
                self.collection
                    .set_generic_families(generic, core::iter::once(id));
            })
    }

    /// Sets the serif generic family mapping.
    pub fn set_serif_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Serif, family_name)
    }

    /// Sets the sans-serif generic family mapping.
    pub fn set_sans_serif_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::SansSerif, family_name)
    }

    /// Sets the cursive generic family mapping.
    pub fn set_cursive_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Cursive, family_name)
    }

    /// Sets the fantasy generic family mapping.
    pub fn set_fantasy_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Fantasy, family_name)
    }

    /// Sets the monospace generic family mapping.
    pub fn set_monospace_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Monospace, family_name)
    }

    /// Sets the system-ui generic family mapping.
    pub fn set_system_ui_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::SystemUi, family_name)
    }

    /// Sets the ui-serif generic family mapping.
    pub fn set_ui_serif_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::UiSerif, family_name)
    }

    /// Sets the ui-sans-serif generic family mapping.
    pub fn set_ui_sans_serif_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::UiSansSerif, family_name)
    }

    /// Sets the ui-monospace generic family mapping.
    pub fn set_ui_monospace_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::UiMonospace, family_name)
    }

    /// Sets the ui-rounded generic family mapping.
    pub fn set_ui_rounded_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::UiRounded, family_name)
    }

    /// Sets the emoji generic family mapping.
    pub fn set_emoji_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Emoji, family_name)
    }

    /// Sets the math generic family mapping.
    pub fn set_math_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::Math, family_name)
    }

    /// Sets the fangsong generic family mapping.
    pub fn set_fang_song_family(&mut self, family_name: &str) -> Result<(), TextError> {
        self.set_generic_family(GenericFamily::FangSong, family_name)
    }
}

/// Text layout context
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LayoutCx(pub LayoutContext<(u32, FontSmoothing)>);

/// Text scaler context
#[derive(Resource, Default, Deref, DerefMut)]
pub struct ScaleCx(pub ScaleContext);
