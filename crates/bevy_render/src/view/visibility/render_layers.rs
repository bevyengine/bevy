use bevy_ecs::prelude::{Component, ReflectComponent};
use bevy_reflect::Reflect;

type LayerMask = u32;

/// An identifier for a rendering layer.
pub type Layer = u8;

/// Describes which rendering layers an entity belongs to.
///
/// Cameras with this component will only render entities with intersecting
/// layers.
///
/// There are 32 layers numbered `0` - [`TOTAL_LAYERS`](RenderLayers::TOTAL_LAYERS). Entities may
/// belong to one or more layers, or no layer at all.
///
/// The [`Default`] instance of `RenderLayers` contains layer `0`, the first layer.
///
/// An entity with this component without any layers is invisible.
///
/// Entities without this component belong to layer `0`.
#[derive(Component, Copy, Clone, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component, PartialEq)]
pub struct RenderLayers(LayerMask);

impl std::fmt::Debug for RenderLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderLayers")
            .field(&self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl std::iter::FromIterator<Layer> for RenderLayers {
    fn from_iter<T: IntoIterator<Item = Layer>>(i: T) -> Self {
        i.into_iter().fold(Self::none(), |mask, g| mask.with(g))
    }
}

/// Defaults to containing to layer `0`, the first layer.
impl Default for RenderLayers {
    fn default() -> Self {
        RenderLayers::layer(0)
    }
}

impl RenderLayers {
    /// The total number of layers supported.
    pub const TOTAL_LAYERS: usize = std::mem::size_of::<LayerMask>() * 8;

    /// Create a new `RenderLayers` belonging to the given layer.
    pub fn layer(n: Layer) -> Self {
        RenderLayers(0).with(n)
    }

    /// Create a new `RenderLayers` that belongs to all layers.
    pub fn all() -> Self {
        RenderLayers(u32::MAX)
    }

    /// Create a new `RenderLayers` that belongs to no layers.
    pub fn none() -> Self {
        RenderLayers(0)
    }

    /// Create a `RenderLayers` from a list of layers.
    pub fn from_layers(layers: &[Layer]) -> Self {
        layers.iter().copied().collect()
    }

    /// Add the given layer.
    ///
    /// This may be called multiple times to allow an entity to belong
    /// to multiple rendering layers. The maximum layer is `TOTAL_LAYERS - 1`.
    ///
    /// # Panics
    /// Panics when called with a layer greater than `TOTAL_LAYERS - 1`.
    #[must_use]
    pub fn with(mut self, layer: Layer) -> Self {
        assert!(usize::from(layer) < Self::TOTAL_LAYERS);
        self.0 |= 1 << layer;
        self
    }

    /// Removes the given rendering layer.
    ///
    /// # Panics
    /// Panics when called with a layer greater than `TOTAL_LAYERS - 1`.
    #[must_use]
    pub fn without(mut self, layer: Layer) -> Self {
        assert!(usize::from(layer) < Self::TOTAL_LAYERS);
        self.0 &= !(1 << layer);
        self
    }

    /// Get an iterator of the layers.
    pub fn iter(&self) -> impl Iterator<Item = Layer> {
        let total: Layer = std::convert::TryInto::try_into(Self::TOTAL_LAYERS).unwrap();
        let mask = *self;
        (0..total).filter(move |g| RenderLayers::layer(*g).intersects(&mask))
    }

    /// Determine if a `RenderLayers` intersects another.
    ///
    /// `RenderLayers`s intersect if they share any common layers.
    ///
    /// A `RenderLayers` with no layers will not match any other
    /// `RenderLayers`, even another with no layers.
    pub fn intersects(&self, other: &RenderLayers) -> bool {
        (self.0 & other.0) > 0
    }
}

#[cfg(test)]
mod rendering_mask_tests {
    use super::{Layer, RenderLayers};

    #[test]
    fn rendering_mask_sanity() {
        assert_eq!(
            RenderLayers::TOTAL_LAYERS,
            32,
            "total layers is what we think it is"
        );
        assert_eq!(RenderLayers::layer(0).0, 1, "layer 0 is mask 1");
        assert_eq!(RenderLayers::layer(1).0, 2, "layer 1 is mask 2");
        assert_eq!(RenderLayers::layer(0).with(1).0, 3, "layer 0 + 1 is mask 3");
        assert_eq!(
            RenderLayers::layer(0).with(1).without(0).0,
            2,
            "layer 0 + 1 - 0 is mask 2"
        );
        assert!(
            RenderLayers::layer(1).intersects(&RenderLayers::layer(1)),
            "layers match like layers"
        );
        assert!(
            RenderLayers::layer(0).intersects(&RenderLayers(1)),
            "a layer of 0 means the mask is just 1 bit"
        );

        assert!(
            RenderLayers::layer(0)
                .with(3)
                .intersects(&RenderLayers::layer(3)),
            "a mask will match another mask containing any similar layers"
        );

        assert!(
            RenderLayers::default().intersects(&RenderLayers::default()),
            "default masks match each other"
        );

        assert!(
            !RenderLayers::layer(0).intersects(&RenderLayers::layer(1)),
            "masks with differing layers do not match"
        );
        assert!(
            !RenderLayers(0).intersects(&RenderLayers(0)),
            "empty masks don't match"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 2, 16, 30])
                .iter()
                .collect::<Vec<_>>(),
            vec![0, 2, 16, 30],
            "from_layers and get_layers should roundtrip"
        );
        assert_eq!(
            format!("{:?}", RenderLayers::from_layers(&[0, 1, 2, 3])).as_str(),
            "RenderLayers([0, 1, 2, 3])",
            "Debug instance shows layers"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 1, 2]),
            <RenderLayers as std::iter::FromIterator<Layer>>::from_iter(vec![0, 1, 2]),
            "from_layers and from_iter are equivalent"
        )
    }
}
