use crate::view::*;

use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use bevy_utils::warn_once;

use smallvec::SmallVec;
use std::ops::Deref;

/// Records the highest [`RenderLayer`] that can be added to a [`RenderLayers`] before
/// a warning is emitted.
///
/// We issue a warning because [`RenderLayers`] allocates in order to have enough room for a given
/// [`RenderLayer`], which is an index into a growable bitset. Large [`RenderLayer`] values can consume
/// a lot of memory since [`RenderLayers`] and [`InheritedRenderLayers`] are potentially on many entities.
pub const RENDER_LAYERS_WARNING_LIMIT: usize = 1024;

/// Records the highest [`RenderLayer`] that can be added to a [`RenderLayers`] before
/// a panic occurs.
///
/// We panic because [`RenderLayers`] allocates in order to have enough room for a given
/// [`RenderLayer`], which is an index into a growable bitset. Large [`RenderLayer`] values can consume
/// a lot of memory since [`RenderLayers`] and [`InheritedRenderLayers`] are potentially on many entities.
pub const RENDER_LAYERS_PANIC_LIMIT: usize = 1_000_000;

/// The default [`RenderLayer`].
pub static DEFAULT_RENDER_LAYER: RenderLayer = RenderLayer(0);

/// Wraps a specific render layer that can be stored in [`RenderLayers`].
///
/// Stores an index into the [`RenderLayers`] internal bitmask.
//todo: Upper limit policy for render layer indices.
#[derive(Reflect, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Deref, DerefMut)]
#[reflect(Default, PartialEq)]
pub struct RenderLayer(pub usize);

impl RenderLayer {
    /// Returns `true` if equal to [`DEFAULT_RENDER_LAYER`].
    pub fn is_default(&self) -> bool {
        *self == DEFAULT_RENDER_LAYER
    }
}

impl From<usize> for RenderLayer {
    fn from(layer: usize) -> Self {
        Self(layer)
    }
}

impl Default for RenderLayer {
    fn default() -> Self {
        DEFAULT_RENDER_LAYER
    }
}

/// Component on an entity that controls which cameras can see it.
///
/// Records a growable bitmask of flags.
/// Individual render layers can be defined with [`RenderLayer`], which is an index
/// into the internal bitmask.
///
/// `RenderLayers::default()` starts with [`DEFAULT_RENDER_LAYER`], which is the global default
/// layer.
///
/// See [`CameraLayer`] for camera-specific details.
///
/// ### Entity default behavior
///
/// All entities without a [`RenderLayers`] component are in [`DEFAULT_RENDER_LAYER`] by
/// default (layer 0). If you add a [`RenderLayers`] component to an entity, it may no longer
/// be in the default layer if the [`RenderLayers`] component doesn't include it.
///
/// For example, if you do `entity.insert(RenderLayers::from(RenderLayer(1)))`, then `entity`
/// will only be in layer 1. You can instead do:
///
/**
```ignore
// Option 1: default
let mut layers = RenderLayers::default();
layers.add(RenderLayer(1));
entity.insert(layers);

// Option 2: explicit
let mut layers = RenderLayers::from(0);
layers.add(RenderLayer(1));
entity.insert(layers);

// Option 3: manual
let layers = RenderLayers::from_layers(&[0, 1]);
entity.insert(layers);
```
*/
///
/// Similarly, if an entity without [`RenderLayers`] inherits from an entity with [`PropagateRenderLayers`] that
/// doesn't propagate layer 0, then the entity's computed [`InheritedRenderLayers`] won't have layer 0 and the
/// entity won't be visible to layer 0.
///
/// ### Performance
///
/// `RenderLayers` occupies 24 bytes on the stack.
///
/// `RenderLayers` can store up to `RenderLayer(63)` without allocating. Allocations occur in 8-byte
/// increments, so the second allocation will occur after `RenderLayer(127)`, and so on.
///
/// See [`RENDER_LAYERS_WARNING_LIMIT`] and [`RENDER_LAYERS_PANIC_LIMIT`] for `RenderLayers` restrictions.
#[derive(Component, Clone, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct RenderLayers {
    layers: SmallVec<[u64; 1]>,
}

impl RenderLayers {
    /// Makes a new `RenderLayers` with no layers.
    pub fn empty() -> Self {
        Self {
            layers: SmallVec::default(),
        }
    }

    /// Makes a new `RenderLayers` from a single [`RenderLayer`].
    pub fn from_layer(layer: impl Into<RenderLayer>) -> Self {
        let mut layers = Self {
            layers: SmallVec::default(),
        };
        layers.add(layer);
        layers
    }

    /// Makes a new `RenderLayers` from a slice.
    pub fn from_layers<T: Into<RenderLayer> + Copy>(layers: &[T]) -> Self {
        layers.iter().map(|l| (*l).into()).collect()
    }

    /// Adds a [`RenderLayer`].
    pub fn add(&mut self, layer: impl Into<RenderLayer>) -> &mut Self {
        let (buffer_index, bit) = Self::layer_info(*(layer.into()));
        self.extend_buffer(buffer_index + 1);
        self.layers[buffer_index] |= bit;
        self
    }

    /// Removes a [`RenderLayer`].
    ///
    /// Does not shrink the internal buffer even if doing so is possible after
    /// removing the layer. We assume if you added a large layer then it is
    /// possible you may re-add another large layer.
    pub fn remove(&mut self, layer: impl Into<RenderLayer>) -> &mut Self {
        let (buffer_index, bit) = Self::layer_info(*(layer.into()));
        if buffer_index >= self.layers.len() {
            return self;
        }
        self.layers[buffer_index] &= !bit;
        self
    }

    /// Clears all stored render layers without deallocating.
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Copies `other` into `Self`.
    ///
    /// This is more efficient than cloning `other` if you want to reuse a `RenderLayers`
    /// that is potentially allocated.
    pub fn set_from(&mut self, other: &Self) {
        self.layers.clear();
        self.layers.reserve_exact(other.layers.len());
        self.layers.extend_from_slice(other.layers.as_slice());
    }

    /// Merges `other` into `Self`.
    ///
    /// After merging, `Self` will include all set bits from `other` and `Self`.
    ///
    /// Will allocate if necessary to include all set bits of `other`.
    pub fn merge(&mut self, other: &Self) {
        self.extend_buffer(other.layers.len());

        for (self_layer, other_layer) in self.layers.iter_mut().zip(other.layers.iter()) {
            *self_layer |= *other_layer;
        }
    }

    /// Gets the number of stored layers.
    ///
    /// Equivalent to `self.iter().count()`.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// Iterates the internal render layers.
    pub fn iter(&self) -> impl Iterator<Item = RenderLayer> + '_ {
        self.layers.iter().copied().flat_map(Self::iter_layers)
    }

    /// Returns `true` if the specified render layer is included in this `RenderLayers`.
    pub fn contains(&self, layer: impl Into<RenderLayer>) -> bool {
        let (buffer_index, bit) = Self::layer_info(*(layer.into()));
        if buffer_index >= self.layers.len() {
            return false;
        }
        (self.layers[buffer_index] & bit) != 0
    }

    /// Returns `true` if `Self` and `other` contain any matching layers.
    pub fn intersects(&self, other: &Self) -> bool {
        for (self_layer, other_layer) in self.layers.iter().zip(other.layers.iter()) {
            if (*self_layer & *other_layer) != 0 {
                return true;
            }
        }

        false
    }

    /// Returns `true` if `Self` intersects with an [`ExtractedRenderLayers`].
    ///
    /// If `extracted` is `None`, then intersections is tested using [`RenderLayers::default`].
    pub fn intersects_extracted(&self, extracted: Option<&ExtractedRenderLayers>) -> bool {
        let default_render_layers = RenderLayers::default();
        let layers = extracted.map(|i| &**i).unwrap_or(&default_render_layers);
        self.intersects(layers)
    }

    /// Gets the bitmask representation of the contained layers
    /// as a slice of bitmasks.
    pub fn bits(&self) -> &[u64] {
        self.layers.as_slice()
    }

    /// Returns `true` if the internal bitmask is on the heap.
    pub fn is_allocated(&self) -> bool {
        self.layers.spilled()
    }

    fn layer_info(layer: usize) -> (usize, u64) {
        if layer > RENDER_LAYERS_WARNING_LIMIT {
            warn_once!("RenderLayers encountered a layer {layer} that exceeded the warning limit \
                RENDER_LAYERS_WARNING_LIMIT = {RENDER_LAYERS_WARNING_LIMIT}, you can ignore this message if \
                that is not a bug");
        }
        if layer > RENDER_LAYERS_PANIC_LIMIT {
            panic!("RenderLayers encountered a layer {layer} that exceeded the maximum upper bound on number of \
                layers RENDER_LAYERS_PANIC_LIMIT = {RENDER_LAYERS_PANIC_LIMIT}");
        }

        let buffer_index = layer / 64;
        let bit_index = layer % 64;
        let bit = 1u64 << bit_index;

        (buffer_index, bit)
    }

    fn extend_buffer(&mut self, other_len: usize) {
        let new_size = std::cmp::max(self.layers.len(), other_len);
        self.layers.reserve_exact(new_size - self.layers.len());
        self.layers.resize(new_size, 0u64);
    }

    fn iter_layers(mut buffer: u64) -> impl Iterator<Item = RenderLayer> + 'static {
        let mut layer: usize = 0;
        std::iter::from_fn(move || {
            if buffer == 0 {
                return None;
            }
            let next = buffer.trailing_zeros() + 1;
            buffer >>= next;
            layer += next as usize;
            Some(RenderLayer(layer - 1))
        })
    }
}

impl<R: Into<RenderLayer>> FromIterator<R> for RenderLayers {
    fn from_iter<T: IntoIterator<Item = R>>(i: T) -> Self {
        i.into_iter().fold(Self::empty(), |mut mask, g| {
            mask.add(g);
            mask
        })
    }
}

impl Default for RenderLayers {
    fn default() -> Self {
        Self::from_layer(DEFAULT_RENDER_LAYER)
    }
}

impl std::fmt::Debug for RenderLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderLayers")
            .field(&self.iter().map(|l| *l).collect::<Vec<_>>())
            .finish()
    }
}

/// Stores a [`RenderLayers`] reference or value.
///
/// Useful for unwrapping an optional reference with fallback to a default value.
pub enum RenderLayersRef<'a> {
    None,
    Ref(&'a RenderLayers),
    Val(RenderLayers),
}

impl<'a> RenderLayersRef<'a> {
    /// Moves `self` into `other` if `self` is on the heap and `other` is not.
    ///
    /// Sets self to [`Self::None`].
    ///
    /// Returns `true` if reclamation occurred.
    pub(crate) fn reclaim(&mut self, other: &mut RenderLayers) -> bool {
        match self {
            Self::Val(layers) => {
                if !layers.is_allocated() || other.is_allocated() {
                    return false;
                }
                *other = std::mem::take(layers);
                *self = Self::None;
                true
            }
            _ => false,
        }
    }
}

impl<'a> Deref for RenderLayersRef<'a> {
    type Target = RenderLayers;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a> RenderLayersRef<'a> {
    /// Gets a reference to the internal [`RenderLayers`].
    ///
    /// Panices if in state [`Self::None`].
    pub fn get(&self) -> &RenderLayers {
        match self {
            Self::None => {
                panic!("RenderLayersRef cannot be dereferenced when empty");
            }
            Self::Ref(layers) => layers,
            Self::Val(layers) => layers,
        }
    }
}

/// Stores a [`RenderLayers`] pointer or value.
///
/// Useful as an alternative to [`RenderLayersRef`] when you can't store a reference, for example within a [`Local`]
/// that buffers a cache that is rewritten every system call.
pub enum RenderLayersPtr {
    Ptr(*const RenderLayers),
    Val(RenderLayers),
}

impl RenderLayersPtr {
    /// Gets a reference to the internal [`RenderLayers`].
    ///
    /// # Safety
    /// Safety must be established by the user.
    pub unsafe fn get(&self) -> &RenderLayers {
        match self {
            // SAFETY: Safety is established by the caller.
            Self::Ptr(layers) => unsafe { layers.as_ref().unwrap() },
            Self::Val(layers) => layers,
        }
    }
}

/// Component on camera entities that controls which [`RenderLayer`] is visible to the camera.
///
/// Cameras can see *at most* one [`RenderLayer`].
/// A camera without the `CameraLayer` component will see the [`DEFAULT_RENDER_LAYER`] layer.
/// A camera with [`CameraLayer::empty`] will see no entities.
///
/// Cameras use entities' [`InheritedRenderLayers`] to determine visibility, with a fall-back to the
/// entity's [`RenderLayers`]. If an entity does not have [`InheritedRenderLayers`]
/// or [`RenderLayers`] components, then the camera will only see it if the camera can
/// view the [`DEFAULT_RENDER_LAYER`] layer.
///
/// A default `CameraLayer` will contain [`DEFAULT_RENDER_LAYER`].
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component, Default, PartialEq)]
pub struct CameraLayer {
    layer: Option<RenderLayer>,
}

impl CameraLayer {
    /// Makes a new `CameraLayer` with no visible [`RenderLayer`].
    pub fn empty() -> Self {
        Self { layer: None }
    }

    /// Makes a new `CameraLayer` with a [`RenderLayer`].
    pub fn new(layer: impl Into<RenderLayer>) -> Self {
        Self {
            layer: Some(layer.into()),
        }
    }

    /// Sets the [`RenderLayer`].
    pub fn set(&mut self, layer: impl Into<RenderLayer>) -> &mut Self {
        self.layer = Some(layer.into());
        self
    }

    /// Removes the current [`RenderLayer`].
    ///
    /// The camera will see nothing after this is called.
    pub fn clear(&mut self) {
        self.layer = None;
    }

    /// Returns the current [`RenderLayer`] if there is one.
    pub fn layer(&self) -> Option<RenderLayer> {
        self.layer
    }

    /// Returns `true` if the specified render layer equals this `CameraLayer`.
    pub fn equals(&self, layer: impl Into<RenderLayer>) -> bool {
        self.layer == Some(layer.into())
    }

    /// Returns `true` if the entity with the specified [`RenderLayers`] is visible
    /// to the `camera` that has this `CameraLayer`.
    pub fn entity_is_visible(&self, layers: &RenderLayers) -> bool {
        let Some(layer) = self.layer else {
            return false;
        };
        layers.contains(layer)
    }

    /// Converts the internal [`RenderLayer`] into a [`RenderLayers`].
    ///
    /// Returns an empty [`RenderLayers`] if there is no stored layer.
    pub fn get_layers(&self) -> RenderLayers {
        match self.layer {
            Some(layer) => RenderLayers::from_layer(layer),
            None => RenderLayers::empty(),
        }
    }
}

impl Default for CameraLayer {
    /// Equivalent to `Self::new(DEFAULT_RENDER_LAYER)`.
    fn default() -> Self {
        Self::new(DEFAULT_RENDER_LAYER)
    }
}

#[cfg(test)]
mod rendering_mask_tests {
    use super::{RenderLayer, RenderLayers, DEFAULT_RENDER_LAYER};
    use smallvec::SmallVec;

    #[test]
    fn rendering_mask_sanity() {
        assert_eq!(
            RenderLayers::default().len(),
            1,
            "default layer contains only one layer"
        );
        assert!(
            RenderLayers::default().contains(DEFAULT_RENDER_LAYER),
            "default layer contains default"
        );
        assert_eq!(
            RenderLayers::from_layer(1).len(),
            1,
            "from contains 1 layer"
        );
        assert!(
            RenderLayers::from_layer(1).contains(RenderLayer(1)),
            "contains is accurate"
        );
        assert!(
            !RenderLayers::from_layer(1).contains(RenderLayer(2)),
            "contains fails when expected"
        );

        assert_eq!(
            RenderLayers::from_layer(0).add(1).layers[0],
            3,
            "layer 0 + 1 is mask 3"
        );
        assert_eq!(
            RenderLayers::from_layer(0).add(1).remove(0).layers[0],
            2,
            "layer 0 + 1 - 0 is mask 2"
        );
        assert!(
            RenderLayers::from_layer(1).intersects(&RenderLayers::from_layer(1)),
            "layers match like layers"
        );
        assert!(
            RenderLayers::from_layer(0).intersects(&RenderLayers {
                layers: SmallVec::from_slice(&[1])
            }),
            "a layer of 0 means the mask is just 1 bit"
        );

        assert!(
            RenderLayers::from_layer(0)
                .add(3)
                .intersects(&RenderLayers::from_layer(3)),
            "a mask will match another mask containing any similar layers"
        );

        assert!(
            RenderLayers::default().intersects(&RenderLayers::default()),
            "default masks match each other"
        );

        assert!(
            !RenderLayers::from_layer(0).intersects(&RenderLayers::from_layer(1)),
            "masks with differing layers do not match"
        );
        assert!(
            !RenderLayers::empty().intersects(&RenderLayers::empty()),
            "empty masks don't match"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 2, 16, 30])
                .iter()
                .collect::<Vec<_>>(),
            vec![
                RenderLayer(0),
                RenderLayer(2),
                RenderLayer(16),
                RenderLayer(30)
            ],
            "from and get_layers should roundtrip"
        );
        assert_eq!(
            format!("{:?}", RenderLayers::from_layers(&[0, 1, 2, 3])).as_str(),
            "RenderLayers([0, 1, 2, 3])",
            "Debug instance shows layers"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 1, 2]),
            <RenderLayers as FromIterator<usize>>::from_iter(vec![0, 1, 2]),
            "from_layers and from_iter are equivalent"
        );
    }
}
