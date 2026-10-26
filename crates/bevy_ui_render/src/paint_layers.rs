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
pub enum NodePaintLayer {
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

impl NodePaintLayer {
    // The builtin paint layers in their default paint order.
    pub const ALL: [Self; 12] = [
        NodePaintLayer::BoxShadow,
        NodePaintLayer::BackgroundColor,
        NodePaintLayer::Border,
        NodePaintLayer::Gradient,
        NodePaintLayer::BorderGradient,
        NodePaintLayer::Image,
        NodePaintLayer::Material,
        NodePaintLayer::TextSelection,
        NodePaintLayer::Text,
        NodePaintLayer::TextStrikethrough,
        NodePaintLayer::TextCursor,
        NodePaintLayer::InlineImage,
    ];
}

/// List of the local Z layers for node.
/// UI elements are drawn in the order of this list, back-to-front.
/// If the label for a builtin renderable element from `UiSubZlayers` isn't present, that
/// element won't be drawn.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct PaintOrder(pub IndexSet<Interned<dyn PaintLayer>>);

impl<P: PaintLayer> FromIterator<P> for PaintOrder {
    fn from_iter<I: IntoIterator<Item = P>>(layers: I) -> Self {
        Self(layers.into_iter().map(|layer| layer.intern()).collect())
    }
}

impl<P: PaintLayer, const N: usize> From<[P; N]> for PaintOrder {
    fn from(layers: [P; N]) -> Self {
        Self::from_iter(layers)
    }
}

impl Default for PaintOrder {
    fn default() -> Self {
        Self::from(NodePaintLayer::ALL)
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

    /// Inserts a layer immediately before an existing layer.
    /// If the inserted layer already exists in the `PaintOrder`, it is moved.
    /// Returns an error if the before label isn't present in the list of layers.
    pub fn insert_before_layer(
        &mut self,
        existing_layer: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer.intern())?;
        self.0.insert_before(index, new_layer.intern());
        Ok(())
    }

    /// Insert a layer immediately after the given layer.
    /// If the inserted layer already exists in the `PaintOrder`, it is moved.
    /// Returns an error if the after label isn't present in the list.
    pub fn insert_after_layer(
        &mut self,
        existing_layer: impl PaintLayer,
        new_layer: impl PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer.intern())?;
        self.0.insert_before(index + 1, new_layer.intern());
        Ok(())
    }

    /// Removes a layer, preserving the order of the remaining layers.
    /// Returns `true` if the layer was present.
    pub fn remove_layer(&mut self, layer: impl PaintLayer) -> bool {
        self.0.shift_remove(&layer.intern())
    }
}

#[derive(Debug)]
pub enum PaintLayerError {
    UnrecognisedLayer(Interned<dyn PaintLayer>),
}

mod tests {
    use crate::paint_layers::NodePaintLayer;
    use crate::paint_layers::PaintOrder;

    #[test]
    fn default_paint_order_matches_node_paint_layer_order() {
        let paint_order = PaintOrder::default();

        assert_eq!(paint_order.len(), NodePaintLayer::ALL.len());
        for (index, layer) in NodePaintLayer::ALL.into_iter().enumerate() {
            assert_eq!(paint_order.get_layer_index(layer).unwrap(), index);
        }
    }
}
