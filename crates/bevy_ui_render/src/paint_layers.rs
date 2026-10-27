use bevy_derive::Deref;
use bevy_derive::DerefMut;
use bevy_ecs::resource::Resource;
use bevy_reflect::Reflect;
use bevy_render::{extract_resource::ExtractResource, RenderApp};
use indexmap::IndexSet;
use smol_str::SmolStr;

/// Identifies a paint layer by name.
#[derive(Clone, PartialEq, Eq, Hash, Reflect)]
pub struct PaintLayer(SmolStr);

impl PaintLayer {
    /// Construct a new [`PaintLayer`] from a [`SmolStr`].
    pub const fn new(name: SmolStr) -> Self {
        Self(name)
    }

    /// Construct a new [`PaintLayer`] from a static string.
    pub const fn new_static(name: &'static str) -> Self {
        Self(SmolStr::new_static(name))
    }
}

impl core::fmt::Display for PaintLayer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for PaintLayer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PaintLayer({:?})", self.0)
    }
}

/// The paint layer for box shadows.
pub const BOX_SHADOW_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.box_shadow");

/// The paint layer for background colors.
pub const BACKGROUND_COLOR_PAINT_LAYER: PaintLayer =
    PaintLayer::new_static("paint_layer.background_color");

/// The paint layer for borders.
pub const BORDER_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.border");

/// The paint layer for background gradients.
pub const GRADIENT_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.gradient");

/// The paint layer for border gradients.
pub const BORDER_GRADIENT_PAINT_LAYER: PaintLayer =
    PaintLayer::new_static("paint_layer.border_gradient");

/// The paint layer for images.
pub const IMAGE_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.image");

/// The paint layer for UI materials.
pub const MATERIAL_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.material");

/// The paint layer for text selection highlights.
pub const TEXT_SELECTION_PAINT_LAYER: PaintLayer =
    PaintLayer::new_static("paint_layer.text_selection");

/// The paint layer for text.
pub const TEXT_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.text");

/// The paint layer for text strikethroughs.
pub const TEXT_STRIKETHROUGH_PAINT_LAYER: PaintLayer =
    PaintLayer::new_static("paint_layer.text_strikethrough");

/// The paint layer for text cursors.
pub const TEXT_CURSOR_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.text_cursor");

/// The paint layer for inline images.
pub const INLINE_IMAGE_PAINT_LAYER: PaintLayer = PaintLayer::new_static("paint_layer.inline_image");

/// Paint layers for built-in UI elements, ordered from back to front.
const DEFAULT_PAINT_LAYERS: [PaintLayer; 12] = [
    BOX_SHADOW_PAINT_LAYER,
    BACKGROUND_COLOR_PAINT_LAYER,
    BORDER_PAINT_LAYER,
    GRADIENT_PAINT_LAYER,
    BORDER_GRADIENT_PAINT_LAYER,
    IMAGE_PAINT_LAYER,
    MATERIAL_PAINT_LAYER,
    TEXT_SELECTION_PAINT_LAYER,
    TEXT_PAINT_LAYER,
    TEXT_STRIKETHROUGH_PAINT_LAYER,
    TEXT_CURSOR_PAINT_LAYER,
    INLINE_IMAGE_PAINT_LAYER,
];

/// Global list of the paint layers available to UI Nodes.
/// Layers are painted in list order, so each layer appears on top of the layers before it.
/// If the layer for a builtin renderable element isn't present, that element won't be drawn.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource)]
#[extract_app(RenderApp)]
pub struct PaintOrder(pub IndexSet<PaintLayer>);

impl FromIterator<PaintLayer> for PaintOrder {
    fn from_iter<I: IntoIterator<Item = PaintLayer>>(layers: I) -> Self {
        let mut paint_order = Self(IndexSet::new());
        for layer in layers {
            paint_order.insert_top(layer);
        }
        paint_order
    }
}

impl<const N: usize> From<[PaintLayer; N]> for PaintOrder {
    fn from(layers: [PaintLayer; N]) -> Self {
        Self::from_iter(layers)
    }
}

impl Default for PaintOrder {
    fn default() -> Self {
        Self::from(DEFAULT_PAINT_LAYERS)
    }
}

impl PaintOrder {
    /// Get the index of the layer.
    #[inline]
    pub fn get_layer_index(&self, layer: PaintLayer) -> Result<usize, PaintLayerError> {
        self.get_index_of(&layer)
            .ok_or(PaintLayerError::UnrecognizedLayer(layer))
    }

    /// Returns the z-offset corresponding to the given paint layer.
    /// The offsets are in the half-open range `0.0..1.0`, increasing from back to front.
    /// The returned offset should be added to the extracted item's stack index to get the final z value.
    ///
    /// Returns an error if the layer label isn't present in the list.
    pub fn get_offset(&self, layer: PaintLayer) -> Result<f32, PaintLayerError> {
        self.get_layer_index(layer)
            .map(|index| index as f32 / self.len() as f32)
    }

    /// Inserts a layer immediately before an existing layer.
    /// If the inserted layer already exists in the `PaintOrder`, it is moved.
    /// Returns an error if the before label isn't present in the list of layers.
    pub fn insert_before_layer(
        &mut self,
        existing_layer: PaintLayer,
        new_layer: PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer)?;
        self.0.insert_before(index, new_layer);
        Ok(())
    }

    /// Insert a layer immediately after the given layer.
    /// If the inserted layer already exists in the `PaintOrder`, it is moved.
    /// Returns an error if the after label isn't present in the list.
    pub fn insert_after_layer(
        &mut self,
        existing_layer: PaintLayer,
        new_layer: PaintLayer,
    ) -> Result<(), PaintLayerError> {
        let index = self.get_layer_index(existing_layer)?;
        self.0.insert_before(index + 1, new_layer);
        Ok(())
    }

    /// Adds a layer at the end of the list. It will be painted last, above everything else.
    /// If the layer is already present, it is moved.
    pub fn insert_top(&mut self, layer: PaintLayer) {
        self.0.insert_before(self.len(), layer);
    }

    /// Adds a layer at the start of the list. It will be painted first, behind everything else.
    /// If the layer is already present, it is moved.
    pub fn insert_bottom(&mut self, layer: PaintLayer) {
        self.0.insert_before(0, layer);
    }

    /// Removes a layer, preserving the order of the remaining layers.
    /// Returns `true` if the layer was present.
    pub fn remove_layer(&mut self, layer: PaintLayer) -> bool {
        self.0.shift_remove(&layer)
    }
}

#[derive(Debug)]
pub enum PaintLayerError {
    UnrecognizedLayer(PaintLayer),
}

#[cfg(test)]
mod tests {
    use crate::paint_layers::PaintLayer;
    use crate::paint_layers::PaintOrder;
    use crate::paint_layers::{DEFAULT_PAINT_LAYERS, TEXT_PAINT_LAYER};

    const A: PaintLayer = PaintLayer::new_static("test.a");
    const B: PaintLayer = PaintLayer::new_static("test.b");
    const C: PaintLayer = PaintLayer::new_static("test.c");
    const X: PaintLayer = PaintLayer::new_static("test.x");

    #[track_caller]
    fn assert_order<const N: usize>(paint_order: &PaintOrder, expected: [PaintLayer; N]) {
        let actual: Vec<_> = paint_order.iter().cloned().collect();
        let expected: Vec<_> = expected.into_iter().collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn default_paint_order_matches_node_paint_layer_order() {
        let paint_order = PaintOrder::default();

        assert_eq!(paint_order.len(), DEFAULT_PAINT_LAYERS.len());
        for (index, layer) in DEFAULT_PAINT_LAYERS.into_iter().enumerate() {
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
    fn insert_top_adds_or_moves_layer_to_end() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        paint_order.insert_top(X);
        assert_order(&paint_order, [A, B, C, X]);

        paint_order.insert_top(A);
        assert_order(&paint_order, [B, C, X, A]);
    }

    #[test]
    fn insert_bottom_adds_or_moves_layer_to_start() {
        let mut paint_order = PaintOrder::from([A, B, C]);

        paint_order.insert_bottom(X);
        assert_order(&paint_order, [X, A, B, C]);

        paint_order.insert_bottom(C);
        assert_order(&paint_order, [C, X, A, B]);
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
    fn from_array_accepts_builtin_and_custom_layers() {
        let paint_order = PaintOrder::from([PaintLayer::new_static("paint_layer.text"), A]);

        assert_eq!(paint_order.get_layer_index(TEXT_PAINT_LAYER).unwrap(), 0);
        assert_eq!(paint_order.get_layer_index(A).unwrap(), 1);
    }
}
