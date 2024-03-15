use bevy_ecs::Entity;

use smallvec::SmallVec;

/// The default [`RenderLayer`].
pub const DEFAULT_RENDER_LAYER: RenderLayer = RenderLayer(0);

/// Wraps a specific render layer that can be stored in [`RenderLayers`].
///
/// Stores an index into the [`RenderLayers`] internal bitmask.
//todo: Upper limit policy for render layer indices.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct RenderLayer(pub usize);

impl RenderLayer
{
    /// Returns `true` if equal to [`DEFAULT_RENDER_LAYER`].
    pub fn is_default(&self) -> bool {
        *self == DEFAULT_RENDER_LAYER
    }
}

impl Default for RenderLayer {
    fn default() -> Self {
        Self(DEFAULT_RENDER_LAYER)
    }
}

/// Records a growable bitmask of flags for controlling which entities
/// are visible to which cameras.
///
/// Individual render layers can be defined with [`RenderLayer`], which is an index
/// into the internal `RenderLayers` bitmask.
///
/// `RenderLayers::default()` starts with [`DEFAULT_RENDER_LAYER`], which is the global default
/// layer.
///
/// ### Performance
///
/// `RenderLayers` occupies 24 bytes on the stack.
///
/// `RenderLayers` can store up to `RenderLayer(63)` without allocating. Allocations occur in 8-byte
/// increments, so the second allocation will occur after `RenderLayer(127)`, and so on.
#[derive(Debug, Clone)]
pub struct RenderLayers
{
    layers: SmallVec<[u64; 1]>,
}

impl RenderLayers {
    /// Makes a new `RenderLayers` with no layers.
    pub fn empty() -> Self {
        Self{ layers: SmallVec::default() }
    }

    /// Adds a [`RenderLayer`].
    pub fn add(&mut self, layer: RenderLayer) {
        let (buffer_index, bit) = Self::layer_info(layer);
        self.extend_buffer(buffer_index + 1);
        self.layers[buffer_index] |= bit;
    }

    /// Removes a [`RenderLayer`].
    ///
    /// Does not shrink the internal buffer even if doing so is possible after
    /// removing the layer. We assume if you added a large layer then it is
    /// possible you may re-add another large layer.
    pub fn remove(&mut self, layer: RenderLayer) {
        let (buffer_index, bit) = Self::layer_info(layer);
        if buffer_index >= self.layers.len() {
            return;
        }
        self.layers[buffer_index] &= ~bit;
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
        self.layers.reserve_exact(other.len());
        self.layers.extend_from_slice(other.layers.as_slice());
    }

    /// Merges `other` into `Self`.
    ///
    /// After merging, `Self` will include all set bits from `other` and `Self`.
    ///
    /// Will allocate if necessary to include all set bits of `other`.
    pub fn merge(&mut self, other: &Self) {
        self.extend_buffer(other.len());

        for (self_layer, other_layer) in self.layers
            .iter_mut()
            .zip(other.layers.iter())
        {
            *self_layer |= *other_layer;
        }
    }

    /// Iterates the internal render layers.
    pub fn iter(&self) -> impl Iterator<Item = RenderLayer> + '_ {
        self.layers
            .iter()
            .copied()
            .map(Self::iter_layers)
            .flatten()
    }

    /// Returns `true` if the specified render layer is included in this `RenderLayers`.
    pub fn contains_layer(&self, layer: RenderLayer) -> bool {
        let (buffer_index, bit) = Self::layer_info(layer);
        if buffer_index >= self.layers.len() {
            return false;
        }
        (self.layers[buffer_index] & bit) != 0
    }

    /// Returns `true` if `Self` and `other` contain any matching layers.
    pub fn intersects(&self, other: &Self) -> bool {
        for (self_layer, other_layer) in self.layers
            .iter()
            .zip(other.layers.iter())
        {
            if (*self_layer & *other_layer) != 0 {
                return true;
            }
        }

        false
    }

    fn layer_info(layer: usize) -> (usize, u64) {
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

    fn iter_layers(mut buffer: u64) -> impl Iterator<Item = RenderLayer> + '_ {
        let mut layer = 0;
        std::iter::from_fn(
            move {
                if buffer == 0 {
                    return None;
                }
                let next = buffer.trailing_zeroes() + 1;
                buffer >>= next;
                layer += next;
                Some(layer - 1)
            }
        )
    }
}

impl From<RenderLayer> for RenderLayers {
    fn from(layer: RenderLayer) -> Self {
        let mut layers = Self{ layers: SmallVec::default() };
        layers.add(layer);
        layers
    }
}

impl Default for RenderLayers {
    fn default() -> Self {
        Self::from(DEFAULT_RENDER_LAYER)
    }
}

/// Component on an entity that controls which cameras can see it.
///
/// There are two kinds of render groups:
/// - [`RenderLayers`]: These are grouping categories that many cameras can view (see [`CameraView`]).
/// - *Camera entity*: This is a specific camera that the entity is affiliated with. This is especially
///   useful for UI in combination with [`PropagateRenderGroups`].
///
/// An entity can be a member of multiple [`RenderLayers`] in addition to having a camera affiliation.
///
/// ### Default behavior
///
/// A default-constructed `RenderGroups` will include [`DEFAULT_RENDER_LAYER`].
/// If you don't want that, then use [`Self::empty`], [`Self::new_with_camera`], or
/// [`Self::from::<RenderLayer>`].
///
/// ### Entity default behavior
///
/// All entities without a [`RenderGroups`] component are in [`DEFAULT_RENDER_LAYER`] by
/// default (layer 0). If you add a [`RenderGroups`] component to an entity, it may no longer
/// be in the default layer if the [`RenderGroups`] component doesn't include it.
///
/// For example, if you do `entity.insert(RenderGroups::from(RenderLayer(1)))`, then `entity`
/// will only be in layer 1. You can instead do:
/**
```no-run
// Option 1: default
let mut groups = RenderGroups::default();
groups.add(RenderLayer(1);
entity.insert(groups);

// Option 2: explicit
let mut groups = RenderGroups::from(0);
groups.add(RenderLayer(1);
entity.insert(groups);
```
*/
#[derive(Component, Debug, Clone)]
pub struct RenderGroups
{
    layers: RenderLayers,
    camera: Option<Entity>,
}

impl RenderGroups {
    /// Makes a new `RenderGroups` with no groups.
    pub fn empty() -> Self {
        Self{ layers: RenderLayers::empty(), camera: None }
    }

    /// Makes a new `RenderGroups` with just a camera.
    pub fn new_with_camera(camera: Entity) -> Self {
        Self{ layers: RenderLayers::empty(), camera: Some(camera) }
    }

    /// Adds a [`RenderLayer`].
    ///
    /// See [`RenderLayers::add`].
    pub fn add(&mut self, layer: RenderLayer) -> &mut Self {
        self.layers.add(layer);
        self
    }

    /// Removes a [`RenderLayer`].
    ///
    /// See [`RenderLayers::remove`].
    pub fn remove(&mut self, layer: RenderLayer) -> &mut Self {
        self.layers.remove(layer);
        self
    }

    /// Clears all stored render layers without deallocating, and unsets the camera affiliation.
    pub fn clear(&mut self) {
        self.layers.clear();
        self.camera = None;
    }

    /// Merges `other` into `Self`.
    ///
    /// After merging, `Self` will include all [`RenderLayers`] from `other` and `Self`.
    /// If both `Self` and `other` have a camera affiliation, then the `Self` camera
    /// will be in the merged result. Otherwise the `other` camera will be in the result.
    ///
    /// Will allocate if necessary to include all [`RenderLayers`] of `other`.
    pub fn merge(&mut self, other: &Self) {
        self.layers.merge(&other.layers);
        self.camera = self.camera.or(other.camera);
    }

    /// Copies `other` into `Self.
    ///
    /// This is more efficient than cloning `other` if you want to reuse a `RenderGroups`
    /// that is potentially allocated.
    pub fn set_from(&mut self, other: &Self) {
        self.layers.set_from(&other.layers);
        self.camera = other.camera;
    }

    /// Sets the camera affiliation.
    ///
    /// Returns the previous camera.
    pub fn set_camera(&mut self, camera: Entity) -> Option<Entity> {
        self.camera.replace(Some(camera))
    }

    /// Removes the current camera affiliation.
    ///
    /// Returns the removed camera.
    pub fn remove_camera(&mut self) -> Option<Entity> {
        self.camera.take()
    }

    /// Returns an iterator over [`RenderLayer`].
    pub fn iter_layers(&self) -> Impl Iterator<Item = RenderLayer> + '_ {
        self.layers.iter()
    }

    /// Returns `true` if the specified render layer is included in this
    /// `RenderGroups`.
    pub fn contains_layer(&self, layer: RenderLayer) -> bool {
        self.layers.contains(layer)
    }

    /// Returns `true` if `Self` intersects with `other`.
    ///
    /// Checks both camera affiliation and [`RenderLayers`] intersection.
    pub fn intersects(&self, other: &Self) -> bool {
        if let (Some(a), Some(b)) = (self.camera, other.camera) {
            if a == b {
                return true;
            }
        }
        self.layers.intersects(&other.layers)
    }

    /// Gets the camera affiliation.
    pub fn camera(&self) -> Option<Entity> {
        self.camera
    }
}

impl From<RenderLayer> for RenderGroups {
    /// Makes a new `RenderGroups` from a specific [`RenderLayer`].
    fn from(layer: RenderLayer) -> Self {
        Self{ layers: RenderLayers::from(layer), camera: None }
    }
}

impl From<RenderLayers> for RenderGroups {
    /// Makes a new `RenderGroups` from a [`RenderLayers`].
    fn from(layers: RenderLayers) -> Self {
        Self{ layers, camera: None }
    }
}

impl Default for RenderGroups {
    /// Equivalent to `Self::from(DEFAULT_RENDER_LAYER)`.
    fn default() -> Self {
        Self::from(DEFAULT_RENDER_LAYER)
    }
}

/// Component on an entity computed by merging its [`RenderGroups`] component with
/// a [`RenderGroups`] propagated by the entity's parent via [`PropagateRenderGroups`].
///
/// This is automatically updated in [`PostUpdate`] in the TODO visibility system.
/// The component will be removed if the entity has no [`RenderGroups`] component and no
/// component is propagated.
///
/// ### Merge details
///
/// This will equal the entity's [`RenderGroups`] component if no groups are propagated, and
/// vice versa if a [`RenderGroups`] is propagated and an entity has no [`RenderGroups`] component.
///
/// The merge direction is 'entity_rendergroups.merge(propagated_rendergroups)`
/// (see [`RenderGroups::merge`]).
/// This means `InheritedRenderGroups` will prioritize the entity's affiliated camera
/// over the propagated affiliated camera.
#[derive(Component, Debug, Clone)]
pub struct InheritedRenderGroups(RenderGroups);

/// Component on camera entities that controls which [`RenderLayers`] are visible to
/// the camera.
///
/// A camera will see any entity that satisfies either of these conditions:
/// - The entity is in a [`RenderLayer`] visible to the camera.
/// - The entity has a [`RenderGroups`] component with camera affiliation equal to the camera.
///
/// Cameras use entities' [`InheritedRenderGroups] to determine visibility. If an entity has no
/// [`InheritedRenderGroups`] component, then the camera will only see it if the camera can
/// view the [`DEFAULT_RENDER_LAYER`] layer.
///
/// A camera without the `CameraView` component will see the [`DEFAULT_RENDER_LAYER`]
/// layer, in addition to relevant [`RenderGroups`] camera affiliations.
///
/// A default `CameraView` will include the [`DEFAULT_RENDER_LAYER`].
#[derive(Component, Debug, Clone)]
pub struct CameraView
{
    layers: RenderLayers,
}

impl CameraView {
    /// Makes a new `CameraView` with no visibile [`RenderLayer`].
    pub fn empty() -> Self {
        Self{ layers: RenderLayers::empty() }
    }

    /// Adds a [`RenderLayer`].
    ///
    /// See [`RenderLayers::add`].
    pub fn add(&mut self, layer: RenderLayer) -> &mut Self {
        self.layers.add(layer);
        self
    }

    /// Removes a [`RenderLayer`].
    ///
    /// See [`RenderLayers::remove`].
    pub fn remove(&mut self, layer: RenderLayer) -> &mut Self {
        self.layers.remove(layer);
        self
    }

    /// Clears all stored render layers without deallocating.
    pub fn clear(&mut self) {
        self.layers.clear();
    }

    /// Returns an iterator over [`RenderLayer`].
    pub fn iter_layers(&self) -> Impl Iterator<Item = RenderLayer> + '_ {
        self.layers.iter()
    }

    /// Returns `true` if the specified render layer is included in this `CameraView`.
    pub fn contains_layer(&self, layer: RenderLayer) -> bool {
        self.layers.contains(layer)
    }

    /// Returns `true` if the entity with the specified [`RenderGroups`] is visible
    /// to the `camera` that has this `CameraView`.
    ///
    /// Checks both camera affiliation and [`RenderLayers`] intersection.
    pub fn entity_is_visible(&self, camera: Entity, groups: &RenderGroups) -> bool {
        if Some(camera) == groups.camera {
            return true;
        }
        self.layers.intersects(&groups.layers)
    }

    /// Converts the internal [`RenderLayers`] into a [`RenderGroups`] affiliated
    /// with the camera that has this `CameraView`.
    pub fn get_groups(&self, camera: Entity) -> RenderGroups {
        let mut groups = RenderGroups::from(self.layers.clone());
        groups.set_camera(camera);
        groups
    }
}

impl From<RenderLayer> for CameraView {
    /// Makes a new `CameraView` from a specific [`RenderLayer`].
    fn from(layer: RenderLayer) -> Self {
        Self{ layers: RenderLayers::from(layer) }
    }
}

impl Default for CameraView {
    /// Equivalent to `Self::from(DEFAULT_RENDER_LAYER)`.
    fn default() -> Self {
        Self::from(DEFAULT_RENDER_LAYER)
    }
}

/// Component on an entity that causes it to propagate a [`RenderGroups`] value to its children.
///
/// See [`RenderGroups`] and [`CameraView`].
#[derive(Component)]
pub enum PropagateRenderGroups
{
    /// If the entity has a [`RenderGroups`] component, that value is propagated.
    ///
    /// Otherwise nothing is propagated and no errors are logged.
    Auto,
    /// If the entity has a [`Camera`] component, propagates `RenderGroups::new_with_camera(entity)`.
    ///
    /// Otherwise an error will be logged.
    Camera,
    /// If the entity has a [`Camera`] component and a [`CameraView`] component, propagates
    /// `CameraView::get_groups(entity)`.
    ///
    /// Otherwise an error will be logged.
    CameraWithView,
    /// Propagates a custom [`RenderGroups`].
    Custom(RenderGroups),
}
