use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::define_label;
use bevy_ecs::intern::Interned;
use bevy_ecs::resource::Resource;
use bevy_render::{extract_resource::ExtractResource, RenderApp};
pub use bevy_ui_render_macros::PaintLayer;

define_label!(PaintLayer);

/// The list of paint layers used to render the builtin UI elements.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, PaintLayer)]
pub enum NodePaintLayers {
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

/// List of the local Z layers for node.
/// UI elements are drawn in the order of this list, back-to-front.
/// If the label for a builtin renderable element from `UiSubZlayers` isn't present, that
/// element won't be drawn.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct PaintOrder(pub Vec<Interned<dyn PaintLayer>>);

impl Default for PaintOrder {
    fn default() -> Self {
        Self(vec![
            NodePaintLayers::BoxShadow.intern(),
            NodePaintLayers::BackgroundColor.intern(),
            NodePaintLayers::Border.intern(),
            NodePaintLayers::Gradient.intern(),
            NodePaintLayers::BorderGradient.intern(),
            NodePaintLayers::Image.intern(),
            NodePaintLayers::Material.intern(),
            NodePaintLayers::TextSelection.intern(),
            NodePaintLayers::Text.intern(),
            NodePaintLayers::TextStrikethrough.intern(),
            NodePaintLayers::TextCursor.intern(),
            NodePaintLayers::InlineImage.intern(),
        ])
    }
}

impl PaintOrder {
    /// Get the z-offset for a paint layer.
    /// The offsets are in the half-open range `0.0..1.0`, increasing from back to front.
    /// The returned offset should be added to the extracted item's stack index to get the final z value.
    ///
    /// Returns an error if the layer label isn't present in the list.
    pub fn get_offset(&self, layer: impl PaintLayer) -> Result<f32, PaintLayerError> {
        let layer = layer.intern();
        self.iter()
            .position(|&label| label == layer)
            .map(|index| index as f32 / self.len() as f32)
            .ok_or(PaintLayerError::UnrecognisedLayer)
    }

    /// Insert a new layer immediately before the given layer.
    /// Returns an error if the before label isn't present in the list.
    pub fn insert_before(
        &mut self,
        before: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let before = before.intern();
        let new_layer = new_layer.intern();
        let index = self
            .iter()
            .position(|&label| label == before)
            .ok_or(PaintLayerError::UnrecognisedLayer)?;
        self.insert(index, new_layer.intern());
        Ok(())
    }

    /// Insert a new layer immediately after the given layer.
    /// Returns an error if the after label isn't present in the list.
    pub fn insert_after(
        &mut self,
        after: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let after = after.intern();
        let new_layer = new_layer.intern();
        let index = self
            .iter()
            .position(|&label| label == after)
            .ok_or(PaintLayerError::UnrecognisedLayer)?;
        self.insert(index + 1, new_layer.intern());
        Ok(())
    }
}

pub enum PaintLayerError {
    UnrecognisedLayer,
}
