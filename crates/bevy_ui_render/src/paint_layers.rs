use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::define_label;
use bevy_ecs::intern::Interned;
use bevy_ecs::resource::Resource;
use bevy_render::{extract_resource::ExtractResource, RenderApp};
pub use bevy_ui_render_macros::PaintLayer;
use indexmap::IndexSet;

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

impl NodePaintLayers {
    /// Returns a list of all the builtin paint layers in their default paint order.
    pub fn all() -> [Self; 12] {
        [
            NodePaintLayers::BoxShadow,
            NodePaintLayers::BackgroundColor,
            NodePaintLayers::Border,
            NodePaintLayers::Gradient,
            NodePaintLayers::BorderGradient,
            NodePaintLayers::Image,
            NodePaintLayers::Material,
            NodePaintLayers::TextSelection,
            NodePaintLayers::Text,
            NodePaintLayers::TextStrikethrough,
            NodePaintLayers::TextCursor,
            NodePaintLayers::InlineImage,
        ]
    }
}

/// List of the local Z layers for node.
/// UI elements are drawn in the order of this list, back-to-front.
/// If the label for a builtin renderable element from `UiSubZlayers` isn't present, that
/// element won't be drawn.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct PaintOrder(pub IndexSet<Interned<dyn PaintLayer>>);

impl<'a, P: PaintLayer, I: IntoIterator<Item = &'a P>> From<I> for PaintOrder {
    fn from(layers: I) -> Self {
        Self(layers.into_iter().map(|layer| layer.intern()).collect())
    }
}

impl Default for PaintOrder {
    fn default() -> Self {
        Self::from(NodePaintLayers::all().iter())
    }
}

impl PaintOrder {
    /// Get the index of the layer.
    #[inline]
    pub fn get_layer_index(&self, layer: impl PaintLayer) -> Result<usize, PaintLayerError> {
        let layer = layer.intern();
        self.get_index_of(&layer)
            .ok_or(PaintLayerError::UnrecognisedLayer(layer))
    }

    /// Returns the z-offset corresponding to the given paint layer.
    /// The offsets are in the half-open range `0.0..1.0`, increasing from back to front.
    /// The returned offset should be added to the extracted item's stack index to get the final z value.
    ///
    /// Returns an error if the layer label isn't present in the list.
    pub fn get_offset(&self, layer: impl PaintLayer) -> Result<f32, PaintLayerError> {
        self.get_layer_index(layer.intern())
            .map(|index| index as f32 / self.len() as f32)
    }

    /// Insert a new layer immediately before the given layer.
    /// Returns an error if the before label isn't present in the list of layers.
    pub fn insert_before(
        &mut self,
        existing_layer: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer.intern())?;
        self.shift_insert(index, new_layer.intern());
        Ok(())
    }

    /// Insert a new layer immediately after the given layer.
    /// Returns an error if the after label isn't present in the list.
    pub fn insert_after(
        &mut self,
        existing_layer: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer.intern())?;
        self.shift_insert(index + 1, new_layer.intern());
        Ok(())
    }
}

#[derive(Debug)]
pub enum PaintLayerError {
    UnrecognisedLayer(Interned<dyn PaintLayer>),
}
