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
/// Layers are painted in list order, so each layer appears on top of the layers before it.
/// If the label for a builtin renderable element from `NodePlayerLayer` isn't present, that
/// element won't be drawn.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct PaintOrder(pub IndexSet<Interned<dyn PaintLayer>>);

impl<P: PaintLayer> FromIterator<P> for PaintOrder {
    fn from_iter<I: IntoIterator<Item = P>>(layers: I) -> Self {
        let mut paint_order = Self(IndexSet::new());
        for layer in layers {
            paint_order.insert_top(layer.intern());
        }
        paint_order
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

    /// Adds a layer at the end of the list. It will be painted last, above everything else.
    /// If the layer is already present, it is moved.
    pub fn insert_top(&mut self, layer: impl PaintLayer) {
        self.0.insert_before(self.len(), layer.intern());
    }

    /// Adds a layer at the start of the list. It will be painted first, behind everything else.
    /// If the layer is already present, it is moved.
    pub fn insert_bottom(&mut self, layer: impl PaintLayer) {
        self.0.insert_before(0, layer.intern());
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

#[cfg(test)]
mod tests {
    use crate::paint_layers::NodePaintLayer;
    use crate::paint_layers::PaintLayer;
    use crate::paint_layers::PaintOrder;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, PaintLayer)]
    enum TestLayer {
        A,
        B,
        C,
        X,
    }

    use TestLayer::*;

    #[track_caller]
    fn assert_order<const N: usize>(paint_order: &PaintOrder, expected: [TestLayer; N]) {
        let actual: Vec<_> = paint_order.iter().copied().collect();
        let expected: Vec<_> = expected.into_iter().map(|layer| layer.intern()).collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn default_paint_order_matches_node_paint_layer_order() {
        let paint_order = PaintOrder::default();

        assert_eq!(paint_order.len(), NodePaintLayer::ALL.len());
        for (index, layer) in NodePaintLayer::ALL.into_iter().enumerate() {
            assert_eq!(paint_order.get_layer_index(layer).unwrap(), index);
        }
    }

    #[test]
    fn insert_after_layer_moves_existing_layer_to_end() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        paint_order.insert_after_layer(C, A).unwrap();

        assert_order(&paint_order, [B, C, A]);
    }

    #[test]
    fn insert_after_layer_moves_existing_layer_backward() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        paint_order.insert_after_layer(A, C).unwrap();

        assert_order(&paint_order, [A, C, B]);
    }

    #[test]
    fn insert_layer_relative_to_itself_does_nothing() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        paint_order.insert_before_layer(B, B).unwrap();
        paint_order.insert_after_layer(B, B).unwrap();

        assert_order(&paint_order, [A, B, C]);
    }

    #[test]
    fn insert_relative_to_missing_layer_is_error() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        assert!(paint_order.insert_before_layer(X, A).is_err());
        assert!(paint_order.insert_after_layer(X, A).is_err());

        assert_order(&paint_order, [A, B, C]);
    }

    #[test]
    fn remove_layer_preserves_order() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        assert!(paint_order.remove_layer(A));

        assert_order(&paint_order, [B, C]);
    }

    #[test]
    fn remove_missing_layer_returns_false() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        assert!(!paint_order.remove_layer(X));

        assert_order(&paint_order, [A, B, C]);
    }

    #[test]
    fn from_array_keeps_second_of_duplicate_layers() {
        let paint_order = PaintOrder::from([A, B, A]);

        assert_order(&paint_order, [B, A]);
    }

    #[test]
    fn from_array_accepts_mixed_label_types() {
        let paint_order = PaintOrder::from([NodePaintLayer::Text.intern(), A.intern()]);

        assert_eq!(
            paint_order.get_layer_index(NodePaintLayer::Text).unwrap(),
            0
        );
        assert_eq!(paint_order.get_layer_index(A).unwrap(), 1);
    }
}
