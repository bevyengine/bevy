use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::intern::Interned;
use bevy_ecs::resource::Resource;
use bevy_render::extract_resource::ExtractResource;
use bevy_render::RenderApp;
pub use bevy_ui_render_macros::UiSubZLayer;

define_label!(UiSubZLayer);

/// Sub layers used by Bevy UI's builtin renderable elements
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, UiSubZLayer)]
pub enum UiSubZLayers {
    BoxShadow,
    BackgroundColor,
    Border,
    Gradient,
    BorderGradient,
    Image,
    Material,
    TextSelection,
    Text,
    TextStrikethrough,
    TextCursor,
    InlineImage,
}

#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct UiSubZLayerList(pub Vec<Interned<dyn UiSubZLayer>>);

impl Default for UiSubZLayerList {
    fn default() -> Self {
        Self(vec![
            UiSubZLayers::BoxShadow.intern(),
            UiSubZLayers::BackgroundColor.intern(),
            UiSubZLayers::Border.intern(),
            UiSubZLayers::Gradient.intern(),
            UiSubZLayers::BorderGradient.intern(),
            UiSubZLayers::Image.intern(),
            UiSubZLayers::Material.intern(),
            UiSubZLayers::TextSelection.intern(),
            UiSubZLayers::Text.intern(),
            UiSubZLayers::TextStrikethrough.intern(),
            UiSubZLayers::TextCursor.intern(),
            UiSubZLayers::InlineImage.intern(),
        ])
    }
}

impl UiSubZLayerList {
    pub fn get_offset(&self, layer: impl UiSubZLayer) -> Result<f32, UiSubZLayerError> {
        let layer = layer.intern();
        self.iter()
            .position(|&label| label == layer)
            .map_err(UiSubZLayerError::NoSuchLayer, |index| {
                index as f32 / self.len() as f32
            })
    }

    pub fn insert_before(
        &mut self,
        before: impl UiSubZLayer,
        new_layer: impl UiSubZLayer,
    ) -> Result<(), UiSubZLayerError> {
        let before = before.intern();
        let new_layer = new_layer.intern();
        self.iter().position(|&label| label == before);
        self.insert(index, layer.intern());
        Ok(())
    }

    pub fn insert_after(
        &mut self,
        after: impl UiSubZLayer,
        new_layer: impl UiSubZLayer,
    ) -> Result<(), UiSubZLayerError> {
        let before = before.intern();
        let new_layer = new_layer.intern();
        self.iter().position(|&label| label == after);
        self.insert(index + 1, layer.intern());
        Ok(())
    }
}

pub enum UiSubZLayerError {
    NoSuchLayer,
}
