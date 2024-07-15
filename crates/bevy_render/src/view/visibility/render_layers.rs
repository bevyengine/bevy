use bevy_ecs::prelude::{Component, ReflectComponent};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use smallvec::SmallVec;

pub const DEFAULT_LAYERS: &RenderLayers = &RenderLayers::layer(0);

/// An identifier for a rendering layer.
pub type Layer = usize;

/// Describes which rendering layers an entity belongs to.
///
/// Cameras with this component will only render entities with intersecting
/// layers.
///
/// Entities may belong to one or more layers, or no layer at all.
///
/// The [`Default`] instance of `RenderLayers` contains layer `0`, the first layer.
///
/// An entity with this component without any layers is invisible.
///
/// Entities without this component belong to layer `0`.
#[derive(Component, Clone, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component, Default, PartialEq)]
pub struct RenderLayers(SmallVec<[u64; INLINE_BLOCKS]>);

/// The number of memory blocks stored inline
const INLINE_BLOCKS: usize = 1;

impl Default for &RenderLayers {
    fn default() -> Self {
        DEFAULT_LAYERS
    }
}

impl std::fmt::Debug for RenderLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderLayers")
            .field(&self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl FromIterator<Layer> for RenderLayers {
    fn from_iter<T: IntoIterator<Item = Layer>>(i: T) -> Self {
        i.into_iter().fold(Self::none(), |mask, g| mask.with(g))
    }
}

impl Default for RenderLayers {
    /// By default, this structure includes layer `0`, which represents the first layer.
    ///
    /// This is distinct from [`RenderLayers::none`], which doesn't belong to any layers.
    fn default() -> Self {
        const { Self::layer(0) }
    }
}

impl RenderLayers {
    /// Create a new `RenderLayers` belonging to the given layer.
    pub const fn layer(n: Layer) -> Self {
        let (buffer_index, bit) = Self::layer_info(n);
        assert!(
            buffer_index < INLINE_BLOCKS,
            "layer is out of bounds for const construction"
        );
        let mut buffer = [0; INLINE_BLOCKS];
        buffer[buffer_index] = bit;
        RenderLayers(SmallVec::from_const(buffer))
    }

    /// Create a new `RenderLayers` that belongs to no layers.
    ///
    /// This is distinct from [`RenderLayers::default`], which belongs to the first layer.
    pub const fn none() -> Self {
        RenderLayers(SmallVec::from_const([0; INLINE_BLOCKS]))
    }

    /// Create a `RenderLayers` from a list of layers.
    pub fn from_layers(layers: &[Layer]) -> Self {
        layers.iter().copied().collect()
    }

    /// Add the given layer.
    ///
    /// This may be called multiple times to allow an entity to belong
    /// to multiple rendering layers.
    #[must_use]
    pub fn with(mut self, layer: Layer) -> Self {
        let (buffer_index, bit) = Self::layer_info(layer);
        self.extend_buffer(buffer_index + 1);
        self.0[buffer_index] |= bit;
        self
    }

    /// Removes the given rendering layer.
    #[must_use]
    pub fn without(mut self, layer: Layer) -> Self {
        let (buffer_index, bit) = Self::layer_info(layer);
        if buffer_index < self.0.len() {
            self.0[buffer_index] &= !bit;
            // Drop trailing zero memory blocks.
            // NOTE: This is not just an optimization, it is necessary for the derived PartialEq impl to be correct.
            if buffer_index == self.0.len() - 1 {
                self = self.shrink();
            }
        }
        self
    }

    /// Get an iterator of the layers.
    pub fn iter(&self) -> impl Iterator<Item = Layer> + '_ {
        self.0.iter().copied().zip(0..).flat_map(Self::iter_layers)
    }

    /// Determine if a `RenderLayers` intersects another.
    ///
    /// `RenderLayers`s intersect if they share any common layers.
    ///
    /// A `RenderLayers` with no layers will not match any other
    /// `RenderLayers`, even another with no layers.
    pub fn intersects(&self, other: &RenderLayers) -> bool {
        // Check for the common case where the view layer and entity layer
        // both point towards our default layer.
        if self.0.as_ptr() == other.0.as_ptr() {
            return true;
        }

        for (self_layer, other_layer) in self.0.iter().zip(other.0.iter()) {
            if (*self_layer & *other_layer) != 0 {
                return true;
            }
        }

        false
    }

    /// get the bitmask representation of the contained layers
    pub fn bits(&self) -> &[u64] {
        self.0.as_slice()
    }

    const fn layer_info(layer: usize) -> (usize, u64) {
        let buffer_index = layer / 64;
        let bit_index = layer % 64;
        let bit = 1u64 << bit_index;

        (buffer_index, bit)
    }

    fn extend_buffer(&mut self, other_len: usize) {
        let new_size = std::cmp::max(self.0.len(), other_len);
        self.0.reserve_exact(new_size - self.0.len());
        self.0.resize(new_size, 0u64);
    }

    fn iter_layers(buffer_and_offset: (u64, usize)) -> impl Iterator<Item = Layer> + 'static {
        let (mut buffer, mut layer) = buffer_and_offset;
        layer *= 64;
        std::iter::from_fn(move || {
            if buffer == 0 {
                return None;
            }
            let next = buffer.trailing_zeros() + 1;
            buffer = buffer.checked_shr(next).unwrap_or(0);
            layer += next as usize;
            Some(layer - 1)
        })
    }

    /// Returns the set of [layers](Layer) shared by two instances of [`RenderLayers`].
    ///
    /// This corresponds to the `self & other` operation.
    pub fn intersection(&self, other: &Self) -> Self {
        self.combine_blocks(other, |a, b| a & b).shrink()
    }

    /// Returns all [layers](Layer) included in either instance of [`RenderLayers`].
    ///
    /// This corresponds to the `self | other` operation.
    pub fn union(&self, other: &Self) -> Self {
        self.combine_blocks(other, |a, b| a | b) // doesn't need to be shrunk, if the inputs are nonzero then the result will be too
    }

    /// Returns all [layers](Layer) included in exactly one of the instances of [`RenderLayers`].
    ///
    /// This corresponds to the "exclusive or" (XOR) operation: `self ^ other`.
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.combine_blocks(other, |a, b| a ^ b).shrink()
    }

    /// Deallocates any trailing-zero memory blocks from this instance
    fn shrink(mut self) -> Self {
        let mut any_dropped = false;
        while self.0.len() > INLINE_BLOCKS && self.0.last() == Some(&0) {
            self.0.pop();
            any_dropped = true;
        }
        if any_dropped && self.0.len() <= INLINE_BLOCKS {
            self.0.shrink_to_fit();
        }
        self
    }

    /// Creates a new instance of [`RenderLayers`] by applying a function to the memory blocks
    /// of self and another instance.
    ///
    /// If the function `f` might return `0` for non-zero inputs, you should call [`Self::shrink`]
    /// on the output to ensure that there are no trailing zero memory blocks that would break
    /// this type's equality comparison.
    fn combine_blocks(&self, other: &Self, mut f: impl FnMut(u64, u64) -> u64) -> Self {
        let mut a = self.0.iter();
        let mut b = other.0.iter();
        let mask = std::iter::from_fn(|| {
            let a = a.next().copied();
            let b = b.next().copied();
            if a.is_none() && b.is_none() {
                return None;
            }
            Some(f(a.unwrap_or_default(), b.unwrap_or_default()))
        });
        Self(mask.collect())
    }
}

impl std::ops::BitAnd for RenderLayers {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(&rhs)
    }
}

impl std::ops::BitOr for RenderLayers {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(&rhs)
    }
}

impl std::ops::BitXor for RenderLayers {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        self.symmetric_difference(&rhs)
    }
}

#[cfg(test)]
mod rendering_mask_tests {
    use super::{Layer, RenderLayers};
    use smallvec::SmallVec;

    #[test]
    fn rendering_mask_sanity() {
        let layer_0 = RenderLayers::layer(0);
        assert_eq!(layer_0.0.len(), 1, "layer 0 is one buffer");
        assert_eq!(layer_0.0[0], 1, "layer 0 is mask 1");
        let layer_1 = RenderLayers::layer(1);
        assert_eq!(layer_1.0.len(), 1, "layer 1 is one buffer");
        assert_eq!(layer_1.0[0], 2, "layer 1 is mask 2");
        let layer_0_1 = RenderLayers::layer(0).with(1);
        assert_eq!(layer_0_1.0.len(), 1, "layer 0 + 1 is one buffer");
        assert_eq!(layer_0_1.0[0], 3, "layer 0 + 1 is mask 3");
        let layer_0_1_without_0 = layer_0_1.without(0);
        assert_eq!(
            layer_0_1_without_0.0.len(),
            1,
            "layer 0 + 1 - 0 is one buffer"
        );
        assert_eq!(layer_0_1_without_0.0[0], 2, "layer 0 + 1 - 0 is mask 2");
        let layer_0_2345 = RenderLayers::layer(0).with(2345);
        assert_eq!(layer_0_2345.0.len(), 37, "layer 0 + 2345 is 37 buffers");
        assert_eq!(layer_0_2345.0[0], 1, "layer 0 + 2345 is mask 1");
        assert_eq!(
            layer_0_2345.0[36], 2199023255552,
            "layer 0 + 2345 is mask 2199023255552"
        );
        assert!(
            layer_0_2345.intersects(&layer_0),
            "layer 0 + 2345 intersects 0"
        );
        assert!(
            RenderLayers::layer(1).intersects(&RenderLayers::layer(1)),
            "layers match like layers"
        );
        assert!(
            RenderLayers::layer(0).intersects(&RenderLayers(SmallVec::from_const([1]))),
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
            !RenderLayers::none().intersects(&RenderLayers::none()),
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
            <RenderLayers as FromIterator<Layer>>::from_iter(vec![0, 1, 2]),
            "from_layers and from_iter are equivalent"
        );

        let tricky_layers = vec![0, 5, 17, 55, 999, 1025, 1026];
        let layers = RenderLayers::from_layers(&tricky_layers);
        let out = layers.iter().collect::<Vec<_>>();
        assert_eq!(tricky_layers, out, "tricky layers roundtrip");
    }

    const MANY: RenderLayers = RenderLayers(SmallVec::from_const([u64::MAX]));

    #[test]
    fn render_layer_ops() {
        let a = RenderLayers::from_layers(&[2, 4, 6]);
        let b = RenderLayers::from_layers(&[1, 2, 3, 4, 5]);

        assert_eq!(
            a.clone() | b.clone(),
            RenderLayers::from_layers(&[1, 2, 3, 4, 5, 6])
        );
        assert_eq!(a.clone() & b.clone(), RenderLayers::from_layers(&[2, 4]));
        assert_eq!(a ^ b, RenderLayers::from_layers(&[1, 3, 5, 6]));

        assert_eq!(RenderLayers::none() & MANY, RenderLayers::none());
        assert_eq!(RenderLayers::none() | MANY, MANY);
        assert_eq!(RenderLayers::none() ^ MANY, MANY);
    }

    #[test]
    fn render_layer_shrink() {
        // Since it has layers greater than 64, the instance should take up two memory blocks
        let layers = RenderLayers::from_layers(&[1, 77]);
        assert!(layers.0.len() == 2);
        // When excluding that layer, it should drop the extra memory block
        let layers = layers.without(77);
        assert!(layers.0.len() == 1);
    }

    #[test]
    fn render_layer_iter_no_overflow() {
        let layers = RenderLayers::from_layers(&[63]);
        layers.iter().count();
    }
}
