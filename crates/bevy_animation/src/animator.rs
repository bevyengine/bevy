use std::{
    any::{type_name, Any, TypeId},
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt,
};

use anyhow::Result;
use bevy_app::{Events, ManualEventReader};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{curves::Curve, interpolation::Lerp};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_transform::prelude::*;
use fnv::FnvBuildHasher;
use smallvec::{smallvec, SmallVec};
use tracing::warn;

use crate::blending::AnimatorBlending;

pub type TrackBase<T> = Box<dyn Curve<Output = T> + Send + Sync + 'static>;

// TODO: Load Clip name from gltf

// Starting from Intel's Sandy Bridge, spatial prefetcher is now pulling pairs of 64-byte cache
// lines at a time, so we have to align to 128 bytes rather than 64.
//
// Sources:
// - https://www.intel.com/content/dam/www/public/us/en/documents/manuals/64-ia-32-architectures-optimization-manual.pdf
// - https://github.com/facebook/folly/blob/1b5288e6eea6df074758f877c849b6e73bbb9fbb/folly/lang/Align.h#L107
//
// ARM's big.LITTLE architecture has asymmetric cores and "big" cores have 128 byte cache line size
// Sources:
// - https://www.mono-project.com/news/2016/09/12/arm64-icache/
//
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
const CACHE_SIZE: usize = 128;
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
const CACHE_SIZE: usize = 64;

/// Defines the number of keyframes that fits inside a cache line;
const KEYFRAMES_PER_CACHE: usize = CACHE_SIZE / std::mem::size_of::<u16>();

///////////////////////////////////////////////////////////////////////////////

pub struct Tracks<T> {
    outputs: SmallVec<[u16; 8]>,
    /// Tuple of track and it's index
    tracks: Vec<(usize, TrackBase<T>)>,
}

impl<T> fmt::Debug for Tracks<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Tracks").field(&self.len()).finish()
    }
}

impl<T> Tracks<T> {
    /// Possible expensive function that calculates from scratch the total tracks duration;
    ///
    /// **NOTE** Caching the result value is desired
    fn calculate_duration(&self) -> f32 {
        self.tracks
            .iter()
            .map(|(_, track)| track.duration())
            .fold(0.0f32, |acc, d| acc.max(d))
    }

    /// Number of curves inside
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (u16, &(usize, TrackBase<T>))> {
        self.outputs.iter().copied().zip(self.tracks.iter())
    }
}

pub struct TrackMeta(&'static str, TypeId);

impl TrackMeta {
    pub fn of<T: 'static>() -> Self {
        Self(type_name::<T>(), TypeId::of::<T>())
    }

    pub const fn type_name(&self) -> &str {
        self.0
    }

    pub const fn type_id(&self) -> TypeId {
        self.1
    }
}

impl fmt::Debug for TrackMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TrackMeta").field(&self.0).finish()
    }
}

#[derive(Debug)]
pub struct TracksUntyped {
    /// Cached calculated curves duration
    duration: f32,
    meta: TrackMeta,
    untyped: Box<dyn Any + Send + Sync + 'static>,
}

impl TracksUntyped {
    fn new<T: Send + Sync + 'static>(tracks: Tracks<T>) -> Self {
        TracksUntyped {
            duration: tracks.calculate_duration(),
            meta: TrackMeta::of::<T>(),
            untyped: Box::new(tracks),
        }
    }

    pub const fn meta(&self) -> &TrackMeta {
        &self.meta
    }

    pub const fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&Tracks<T>> {
        self.untyped.downcast_ref()
    }

    #[inline]
    pub fn downcast_mut<T: 'static>(&mut self) -> Option<&mut Tracks<T>> {
        self.untyped.downcast_mut()
    }
}

// TODO: impl Serialize, Deserialize using bevy reflect for that
// the hierarchy, this animated entity asset must match the one been used in the animator
// this could help retarget animations, but will mess the clip
#[derive(Debug, TypeUuid)]
#[uuid = "79e2ea58-8bf7-43af-8219-5898edb02f80"]
pub struct Clip {
    pub name: String,
    /// Should this clip loop (warping around) or hold
    ///
    /// **NOTE** Keep in mind that sampling with time greater
    /// than the clips duration will always hold.
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    /// Clip compound duration
    duration: f32,
    // ! FIXME: Limit to only entities that have animations to avoid
    // ! fetching extra components that aren't been used
    hierarchy: NamedHierarchy,
    // ? NOTE: AHash performed worse than FnvHasher
    // TODO: Change to the hashbrown::raw::RawTable and use a `label!()` to make hashes constants
    /// Each curve and keyframe cache index mapped by property name
    properties: HashMap<String, (usize, TracksUntyped), FnvBuildHasher>,
    /// Number of animated properties
    len: usize,
    /// Number of cache lines currently been used to organize the keyframe
    /// caching into buckets to be accessed by many different threads at the same time
    keyframes_cache_buckets: usize,
}

// fn clip_default_warp() -> bool {
//     true
// }

impl Default for Clip {
    fn default() -> Self {
        Self {
            name: String::new(),
            warp: true,
            duration: 0.0,
            hierarchy: NamedHierarchy::default(),
            properties: HashMap::default(),
            len: 0,
            keyframes_cache_buckets: 0,
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    ///
    /// Please use preferably `add_curve` as it's future proof and type safe;
    ///
    /// **NOTE** This is a expensive function;
    ///
    /// **NOTE** You can safely ignore the return value as it's only used for assertions during tests;
    /// The return value is the assigned curve index in cache line bucket.
    pub fn add_track_at_path<T>(&mut self, path: &str, track: T) -> usize
    where
        T: Curve + Send + Sync + 'static,
        <T as Curve>::Output: Lerp + Clone + Send + Sync + 'static,
    {
        // Split in entity and attribute path,
        // NOTE: use rfind because it's expected the latter to be generally shorter
        let path = path.split_at(path.rfind('@').expect("property path missing @"));
        let entity_path = path.0;
        let property_path = path.1.split_at(1).1;

        // Clip an only have some amount of curves and entities
        // this limitation was added to save memory (but you can increase it if you want)
        assert!(
            self.len + 1 < (u16::MAX as usize),
            "clip curve limit reached"
        );

        let (entity_index, _) = self.hierarchy.get_or_insert_entity(entity_path);

        if let Some((cache_index, tracks_untyped)) = self.properties.get_mut(property_path) {
            let tracks = tracks_untyped
                .downcast_mut::<<T as Curve>::Output>()
                .expect("properties can't have the same name and different curve types");

            let search = tracks
                .outputs
                .iter()
                .position(|index| *index == entity_index);

            // If some entity was created it means this property is a new one so we can safely skip the attribute testing
            if let Some(track_index) = search {
                let (kf_index, source) = &mut tracks.tracks[track_index];
                let k_index = *kf_index;

                let mut track: TrackBase<<T as Curve>::Output> = Box::new(track);

                // Found a property equal to the one been inserted, next replace the curve
                std::mem::swap(source, &mut track);

                // Update curve duration in two stages
                let duration = tracks.calculate_duration();

                // Drop the down casted ref and update the parent curve
                tracks_untyped.duration = duration;

                // Drop the curves untyped (which as a mut borrow) and update the total duration
                self.duration = self.calculate_duration();

                return k_index;
            }

            *cache_index += 1;
            if (*cache_index % KEYFRAMES_PER_CACHE) == 0 {
                // No more spaces left in the current cache line
                *cache_index = self.keyframes_cache_buckets * KEYFRAMES_PER_CACHE;
                self.keyframes_cache_buckets += 1;
            }

            self.len += 1;

            // Append newly added curve
            let duration = track.duration();
            tracks.outputs.push(entity_index);
            tracks.tracks.push((*cache_index, Box::new(track)));

            self.len += 1;
            self.duration = self.duration.max(duration);
            tracks_untyped.duration = tracks_untyped.duration.max(duration);

            return *cache_index;
        }

        let cache_index = self.keyframes_cache_buckets * KEYFRAMES_PER_CACHE;
        self.keyframes_cache_buckets += 1;
        self.len += 1;

        self.duration = self.duration.max(track.duration());
        self.properties.insert(
            property_path.to_string(),
            (
                cache_index,
                TracksUntyped::new(Tracks {
                    outputs: smallvec![entity_index],
                    tracks: vec![(cache_index, Box::new(track))],
                }),
            ),
        );

        cache_index
    }

    /// Number of animated properties in this clip
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // /// Returns the property curve property path.
    // ///
    // /// The clip stores a property path in a specific way to improve search performance
    // /// thus it needs to rebuilt the curve property path in the human readable format
    // pub fn get_property_path(&self, index: u16) -> String {
    //     let CurveEntry {
    //         entity_index,
    //         property_index,
    //     } = &self.entries[index as usize];
    //
    //     format!(
    //         "{}@{}",
    //         self.hierarchy
    //             .get_entity_path_at(*entity_index)
    //             .expect("property as an invalid entity"),
    //         self.properties[*property_index as usize].as_str()
    //     )
    // }

    fn calculate_duration(&self) -> f32 {
        self.properties
            .iter()
            .map(|(_, (_, c))| c.duration)
            .fold(0.0f32, |acc, x| acc.max(x))
    }

    /// Clip duration
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    #[inline]
    pub fn hierarchy(&self) -> &NamedHierarchy {
        &self.hierarchy
    }

    /// Get the curves an given property
    #[inline]
    pub fn get(&self, property_name: &str) -> Option<&TracksUntyped> {
        self.properties
            .get(property_name)
            .map(|(_, curve_untyped)| curve_untyped)
    }
}

///////////////////////////////////////////////////////////////////////////////

#[cfg_attr(any(target_arch = "x86_64", target_arch = "aarch64"), repr(align(128)))]
#[cfg_attr(
    not(any(target_arch = "x86_64", target_arch = "aarch64")),
    repr(align(64))
)]
#[derive(Clone)]
struct KeyframeBucket([u16; KEYFRAMES_PER_CACHE]);

impl Default for KeyframeBucket {
    fn default() -> Self {
        Self([0; KEYFRAMES_PER_CACHE])
    }
}

#[derive(Clone, Reflect)]
pub struct Layer {
    pub weight: f32,
    pub clip: usize,
    pub time: f32,
    pub time_scale: f32,
    pub additive: bool,
    #[reflect(ignore)]
    keyframes_buckets: Vec<KeyframeBucket>,
    // TODO: LayerMask
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            weight: 1.0,
            clip: 0,
            time: 0.0,
            time_scale: 1.0,
            additive: false,
            keyframes_buckets: vec![],
        }
    }
}

impl fmt::Debug for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Layer")
            .field("weight", &self.weight)
            .field("clip", &self.clip)
            .field("time", &self.time)
            .field("time_scale", &self.time_scale)
            .field("additive", &self.additive)
            .finish()
    }
}

impl Layer {
    #[inline]
    pub fn keyframes(&self) -> &[u16] {
        // SAFETY: Keyframes vec still needs to be flatten
        unsafe {
            std::slice::from_raw_parts(
                self.keyframes_buckets.as_ptr() as *const _,
                self.keyframes_buckets.len() * KEYFRAMES_PER_CACHE,
            )
        }
    }

    #[inline]
    pub fn keyframes_mut(&mut self) -> &mut [u16] {
        // SAFETY: Have mutability over self, but the keyframes vec still needs to be flatten
        unsafe { self.keyframes_unsafe() }
    }

    /// # Safety
    /// Ensure no other borrows exist along with this one.
    ///
    /// Alternatively its also possible to use the `curve_index` for properties that are only borrowed once;
    /// The curves indexes are unique for each curve and for different properties are also guaranteed to not share the same cache line,
    /// making possible to access the layer keyframes from multiple threads that doesn't look at the same property at the same time.
    ///
    /// Any race condition should only slowdown the animation sampling,
    /// the `sample_with_cursor` function can recover no matter the input given;
    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn keyframes_unsafe(&self) -> &mut [u16] {
        std::slice::from_raw_parts_mut(
            self.keyframes_buckets.as_ptr() as *mut _,
            self.keyframes_buckets.len() * KEYFRAMES_PER_CACHE,
        )
    }
}

#[derive(Default, Debug)]
struct Bind {
    entity_indexes: Vec<u16>,
}

#[derive(Debug, Reflect)]
#[reflect(Component)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,

    #[reflect(ignore)]
    bind_clips: Vec<Option<Bind>>,
    #[reflect(ignore)]
    hierarchy: NamedHierarchy,
    #[reflect(ignore)]
    missing_entities: bool,
    #[reflect(ignore)]
    entities: Vec<Option<Entity>>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: vec![],
            bind_clips: vec![],
            hierarchy: NamedHierarchy::default(),
            missing_entities: false,
            entities: vec![],
            time_scale: 1.0,
            layers: vec![],
        }
    }
}

impl Animator {
    pub fn add_clip(&mut self, clip: Handle<Clip>) -> usize {
        if let Some(i) = self.clips.iter().position(|c| *c == clip) {
            i
        } else {
            let i = self.clips.len();
            self.clips.push(clip);
            i
        }
    }

    // TODO: remove clip

    pub fn add_layer(&mut self, clip: Handle<Clip>, weight: f32) -> usize {
        let clip = self.add_clip(clip);
        let layer_index = self.layers.len();
        self.layers.push(Layer {
            clip,
            weight,
            ..Default::default()
        });
        layer_index
    }

    // TODO: remove layer at index

    // TODO: cleanup, clears hierarchy entities and binds

    #[inline]
    pub fn entities(&self) -> &[Option<Entity>] {
        &self.entities[..]
    }

    #[inline]
    pub fn clips(&self) -> &[Handle<Clip>] {
        &self.clips[..]
    }

    pub fn animate(&self) -> LayerIterator {
        LayerIterator {
            index: 0,
            animator: self,
        }
    }
}

pub struct LayerIterator<'a> {
    index: usize,
    animator: &'a Animator,
}

impl<'a> Iterator for LayerIterator<'a> {
    type Item = (usize, &'a Layer, &'a Handle<Clip>, &'a [u16]);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get next layer or stop the iterator
            let layer = self.animator.layers.get(self.index)?;
            let index = self.index;
            self.index += 1;

            let clip_index = layer.clip as usize;
            if let Some(clip_handle) = self.animator.clips.get(clip_index) {
                if let Some(Some(bind)) = self.animator.bind_clips.get(clip_index) {
                    return Some((index, layer, clip_handle, &bind.entity_indexes[..]));
                }

                // Missing clip bind continue to the next layer
                // TODO: log error
            }

            // Invalid clip continue to the next layer
            // TODO: log error
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

/// Used to validate animated properties path and types
#[derive(Default)]
pub(crate) struct AnimatorRegistry {
    /// Set of registered animators for components and assets
    pub(crate) targets: HashSet<TypeId, FnvBuildHasher>,
    /// All static properties available
    pub(crate) static_properties: HashSet<(Cow<'static, str>, TypeId), FnvBuildHasher>,
}

/// State info for the system `animator_binding_system`
#[derive(Default)]
pub(crate) struct BindingState {
    clips_event_reader: ManualEventReader<AssetEvent<Clip>>,
    clips_modified: HashSet<Handle<Clip>, FnvBuildHasher>,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn animator_update_system(
    mut commands: Commands,
    mut state: Local<BindingState>,
    clip_events: Res<Events<AssetEvent<Clip>>>,
    animator_registry: Res<AnimatorRegistry>,
    time: Res<Time>,
    clips: Res<Assets<Clip>>,
    mut animators_query: Query<(Entity, &mut Animator, Option<&AnimatorBlending>)>,
    children_query: Query<&Children>,
    name_query: Query<(&Parent, &Name)>,
    parent_or_name_changed_query: Query<
        (Option<&Parent>, &Name),
        Or<(Changed<Parent>, Changed<Name>)>,
    >,
    entity_deleted_query: Query<Entity>,
    parent_or_name_removed_query: Query<Entity, Or<(Without<Parent>, Without<Name>)>>,
) {
    let __span = tracing::info_span!("animator_update_system");
    let __guard = __span.enter();

    // ? NOTE: Changing a clip on fly is supported, but is expensive so use with caution
    // TODO: Put a warning somewhere in the docs on how expensive is changing a clip on the fly
    // Query all clips that changed and remove their binds from the animator
    state.clips_modified.clear();
    for event in state.clips_event_reader.iter(&clip_events) {
        match event {
            AssetEvent::Removed { handle } => state.clips_modified.insert(handle.clone()),
            AssetEvent::Modified { handle } => state.clips_modified.insert(handle.clone()),
            _ => false,
        };
    }

    for (animator_entity, mut animator, animator_blending) in animators_query.iter_mut() {
        let animator = &mut *animator;

        if animator_blending.is_none() {
            commands
                .entity(animator_entity)
                .insert(AnimatorBlending::default());
        }

        // Time scales by component
        let delta_time = time.delta_seconds() * animator.time_scale;

        // Invalidate entities on parent or name changed events
        for (entity_index, ((parent_index, name), _)) in animator.hierarchy.iter().enumerate() {
            // Ignore the root entity as we don't care about it's parent nor it's name
            if entity_index == 0 {
                continue;
            }

            let mut remove_entity = false;
            let entity = animator.entities[entity_index];

            if let Some(entity) = entity {
                // Entity deleted
                // or Parent or Name component where removed from entity
                if entity_deleted_query.get(entity).is_err()
                    || parent_or_name_removed_query.get(entity).is_ok()
                {
                    remove_entity = true;
                } else if let Ok((entity_parent, entity_name)) =
                    parent_or_name_changed_query.get(entity)
                {
                    // Parent or Name changed
                    // ? NOTE: Use a changed events for both parent and name because name
                    // ? comparison isn't fast, as result it won't notice changes before the
                    // ? POST_UPDATE stage

                    let parent = animator.entities[*parent_index as usize];
                    if entity_parent.map(|p| p.0) != parent || name != entity_name {
                        remove_entity = true;
                    }
                }
            }

            // Remove entity
            if remove_entity {
                let animator_entities = &mut animator.entities;
                animator
                    .hierarchy
                    .depth_first(entity_index as u16, &mut |index, _| {
                        animator_entities[index as usize] = None;
                    });
                animator.missing_entities = true;
            }
        }

        // TODO: Figure out how expensive this might be with a couple of entities missing
        // Look for missing entities if any
        if animator.missing_entities {
            let mut missing = false;
            let count = animator.hierarchy.len();

            // Prepare the entities table cache
            animator.entities.resize(count, None);
            // Assign the root entity as the first element
            animator.entities[0] = Some(animator_entity);

            // Find missing entities that where just added in the hierarchy
            for entity_index in 0..count {
                missing |= animator
                    .hierarchy
                    .find_entity(
                        entity_index as u16,
                        &mut animator.entities,
                        &children_query,
                        &name_query,
                    )
                    .is_none();
            }

            // Update missing entities state
            animator.missing_entities = missing;

            // TODO: Better warning messages
            warn!("missing entities");
        }

        // Make run for the binds
        animator
            .bind_clips
            .resize_with(animator.clips.len(), || None);

        for (clip_index, clip_handle) in animator.clips.iter().enumerate() {
            // Invalidate clip binds on clip change
            // NOTE: The clip might be gone so it's necessary to update it here
            if state.clips_modified.contains(clip_handle) {
                animator.bind_clips[clip_index] = None;
                // TODO: warning clip modified while assigned to an animator
            }

            if let Some(clip) = clips.get(clip_handle) {
                let bind_slot = &mut animator.bind_clips[clip_index];
                if bind_slot.is_none() {
                    // TODO: Add option to ignore these extra checks (or not)
                    // Check if the clips properties are registered
                    for (property_name, (_, curves)) in clip.properties.iter() {
                        if !animator_registry.static_properties.contains(&(
                            Cow::Borrowed(property_name.as_str()),
                            curves.meta().type_id(),
                        )) {
                            // TODO: Check dynamic properties names using regex
                            warn!(
                                "unregistered property '{}' of type `{}` in clip '{}', maybe the property name is misspelled or it's type isn't registered as animated",
                                &property_name,
                                curves.meta().type_name(),
                                &clip.name
                            );
                        }
                    }

                    // Merge newly added clip hierarchy into the animator global hierarchy
                    let mut bind = Bind::default();
                    animator
                        .hierarchy
                        .merge(&clip.hierarchy, &mut bind.entity_indexes);

                    // Prepare the entities table cache
                    animator.entities.resize(animator.hierarchy.len(), None);
                    // Assign the root entity as the first element
                    animator.entities[0] = Some(animator_entity);

                    let mut missing = false;
                    // Find missing entities that where just added in the hierarchy
                    for entity_index in &bind.entity_indexes {
                        missing |= animator
                            .hierarchy
                            .find_entity(
                                *entity_index,
                                &mut animator.entities,
                                &children_query,
                                &name_query,
                            )
                            .is_none();
                    }

                    // Set missing entities if any
                    animator.missing_entities |= missing;

                    *bind_slot = Some(bind);
                };

                for layer in &mut animator.layers {
                    if layer.clip as usize != clip_index {
                        continue;
                    }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // Ensure keyframes capacity
                    layer
                        .keyframes_buckets
                        .resize_with(clip.keyframes_cache_buckets, KeyframeBucket::default);

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // Reset keyframes indexes (speedup sampling)
                            layer
                                .keyframes_buckets
                                .iter_mut()
                                .for_each(|k| *k = KeyframeBucket::default());
                        }
                    } else {
                        // Hold
                        time = time.min(clip.duration());
                    }

                    // Advance state time
                    layer.time = time;
                }
            }
        }
    }

    std::mem::drop(__guard);
}

#[cfg(test)]
#[allow(dead_code)]
mod tests {
    use super::*;
    use bevy_asset::AddAsset;
    use bevy_math::{curves::CurveVariableLinear, Quat, Vec3};
    use bevy_pbr::prelude::StandardMaterial;
    use bevy_render::prelude::Mesh;

    struct AnimatorTestBench {
        app: bevy_app::App,
        entities: Vec<Entity>,
    }

    impl AnimatorTestBench {
        fn new() -> Self {
            let mut app_builder = bevy_app::App::build();
            app_builder
                .add_plugin(bevy_core::CorePlugin::default())
                .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
                .add_plugin(bevy_asset::AssetPlugin::default())
                .add_asset::<Mesh>()
                .add_asset::<StandardMaterial>()
                .add_plugin(bevy_transform::TransformPlugin::default())
                .add_plugin(crate::AnimationPlugin { skinning: false });

            let mut world = World::new();
            let base = (
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            );

            // Create animator and assign some clips
            let mut animator = Animator::default();
            {
                let mut clip_a = Clip::default();
                clip_a.add_track_at_path(
                    "@Transform.translation",
                    CurveVariableLinear::from_line(0.0, 1.0, Vec3::X, -Vec3::X),
                );
                let rot = CurveVariableLinear::from_constant(Quat::IDENTITY);
                clip_a.add_track_at_path("@Transform.rotation", rot.clone());
                clip_a.add_track_at_path("/Node1@Transform.rotation", rot.clone());
                clip_a.add_track_at_path("/Node1/Node2@Transform.rotation", rot);

                let mut clip_b = Clip::default();
                clip_b.add_track_at_path(
                    "@Transform.translation",
                    CurveVariableLinear::from_constant(Vec3::ZERO),
                );
                let rot = CurveVariableLinear::from_line(
                    0.0,
                    1.0,
                    Quat::from_axis_angle(Vec3::Z, 0.1),
                    Quat::from_axis_angle(Vec3::Z, -0.1),
                );
                clip_b.add_track_at_path("@Transform.rotation", rot.clone());
                clip_b.add_track_at_path("/Node1@Transform.rotation", rot.clone());
                clip_b.add_track_at_path("/Node1/Node2@Transform.rotation", rot);

                let mut clips = app_builder
                    .world_mut()
                    .get_resource_mut::<Assets<Clip>>()
                    .unwrap();
                let clip_a = clips.add(clip_a);
                let clip_b = clips.add(clip_b);

                animator.add_layer(clip_a, 0.5);
                animator.add_layer(clip_b, 0.5);
            }

            let mut entities = vec![];

            let root = world
                .spawn()
                .insert_bundle(base.clone())
                .insert(Name::new("Root"))
                .insert(animator)
                .id();

            entities.push(root);

            world.entity_mut(root).with_children(|world_builder| {
                entities.push(
                    world_builder
                        .spawn_bundle(base.clone())
                        .insert(Name::new("Node1"))
                        .id(),
                );

                world_builder.with_children(|world_builder| {
                    entities.push(
                        world_builder
                            .spawn()
                            .insert_bundle(base.clone())
                            .insert(Name::new("Node2"))
                            .id(),
                    );

                    world_builder.with_children(|world_builder| {
                        entities.push(
                            world_builder
                                .spawn()
                                .insert_bundle(base.clone())
                                .insert(Name::new("Node3"))
                                .id(),
                        );
                    });
                });
            });

            app_builder.set_world(world);
            app_builder.app.update();

            Self {
                app: app_builder.app,
                entities,
            }
        }

        fn run(&mut self) {
            self.app.update();
        }

        fn animator(&mut self) -> Mut<Animator> {
            self.app
                .world
                .get_mut::<Animator>(self.entities[0])
                .unwrap()
        }
    }

    #[test]
    fn entity_deleted() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        // delete "Node1"
        test_bench.app.world.despawn(test_bench.entities[1]);

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-add "Node1"
        test_bench.entities[1] = test_bench
            .app
            .world
            .spawn()
            .insert_bundle((
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                Name::new("Node1"),
                Parent(test_bench.entities[0]),
            ))
            .id();

        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[2])
            .unwrap() = Parent(test_bench.entities[1]);

        // Tick
        // ! FIXME: May take one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_parent_changed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        // Change parent of node "Node1"
        let new_parent = test_bench.app.world.spawn().id();

        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(new_parent);

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-parent "Node1"
        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(test_bench.entities[0]);

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_parent_component_removed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench
            .app
            .world
            .entity_mut(test_bench.entities[1])
            .remove::<Parent>()
            .unwrap();

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        // Re-parent "Node1"
        test_bench
            .app
            .world
            .entity_mut(test_bench.entities[1])
            .insert(Parent(test_bench.entities[0]));

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_renamed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        *test_bench
            .app
            .world
            .get_mut::<Name>(test_bench.entities[1])
            .unwrap() = Name::new("Spine1");

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        *test_bench
            .app
            .world
            .get_mut::<Name>(test_bench.entities[1])
            .unwrap() = Name::new("Node1");

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn entity_name_component_removed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench
            .app
            .world
            .entity_mut(test_bench.entities[1])
            .remove::<Name>()
            .unwrap();

        // Tick
        test_bench.run();

        assert!(
            test_bench.animator().entities()[0].is_some(),
            "root entity missing"
        );
        assert_eq!(
            &test_bench.animator().entities()[1..],
            &[None, None][..],
            "entities still binded"
        );

        test_bench
            .app
            .world
            .entity_mut(test_bench.entities[1])
            .insert(Name::new("Node1"));

        // Tick
        // ! FIXME: Takes one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn clip_changed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));

        test_bench.run();

        // Modify clip
        {
            let clip_a_handle = test_bench.animator().clips()[0].clone();
            let mut clips = test_bench
                .app
                .world
                .get_resource_mut::<Assets<Clip>>()
                .unwrap();
            let clip_a = clips.get_mut(clip_a_handle).unwrap();
            clip_a.add_track_at_path(
                "/Node3/@Transform.translation",
                CurveVariableLinear::from_line(0.0, 1.0, Vec3::Z, -Vec3::Z),
            );
        }

        test_bench.run();

        assert_eq!(test_bench.animator().entities().len(), 4);

        // Spawn entity
        test_bench.app.world.spawn().insert_bundle((
            GlobalTransform::default(),
            Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Name::new("Node3"),
            Parent(test_bench.entities[0]),
        ));

        test_bench.run();
        test_bench.run();

        test_bench
            .animator()
            .entities()
            .iter()
            .enumerate()
            .for_each(|(index, entity)| assert!(entity.is_some(), "missing entity {}", index,));
    }

    #[test]
    fn clip_property_index_are_grouped_into_cache_line_buckets() {
        let curve = CurveVariableLinear::from_constant(0.0);
        let mut clip = Clip::default();
        assert_eq!(clip.add_track_at_path("a@T.t", curve.clone()), 0);
        assert_eq!(clip.add_track_at_path("b@T.t", curve.clone()), 1);
        assert_eq!(
            clip.add_track_at_path("a@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE
        );
        assert_eq!(clip.add_track_at_path("c@T.t", curve.clone()), 2);
        assert_eq!(
            clip.add_track_at_path("b@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 1
        );
        assert_eq!(
            clip.add_track_at_path("c@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 2
        );
        assert_eq!(clip.add_track_at_path("d@T.t", curve.clone()), 3);

        for i in 4..KEYFRAMES_PER_CACHE {
            assert_eq!(
                clip.add_track_at_path(&format!("node_{}@T.t", i), curve.clone()),
                i
            );
        }

        assert_eq!(
            clip.add_track_at_path("za@T.t", curve.clone()),
            KEYFRAMES_PER_CACHE * 2
        );
        assert_eq!(
            clip.add_track_at_path("za@T.r", curve.clone()),
            KEYFRAMES_PER_CACHE + 3
        );
        assert_eq!(
            clip.add_track_at_path("zb@T.t", curve.clone()),
            KEYFRAMES_PER_CACHE * 2 + 1
        );
    }
}
