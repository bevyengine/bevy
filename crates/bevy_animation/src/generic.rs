use anyhow::Result;
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle /*, HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::{Location, SystemId};
use bevy_math::prelude::*;
use bevy_property::{Properties, Property, PropertyType};
use bevy_transform::prelude::*;
use bevy_type_registry::{TypeRegistry, TypeUuid};
use std::collections::HashSet;
//use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::convert::TryFrom;
use std::hash::Hash;
use std::ptr::null_mut;

use super::lerping::LerpValue;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// TODO: impl Serialize, Deserialize
#[derive(Debug, Clone, TypeUuid)]
#[uuid = "4c76e6c3-706d-4a74-af8e-4f48033e0733"]
pub struct Clip {
    //#[serde(default = "clip_default_warp")]
    pub warp: bool,
    duration: f32,
    /// Entity identification made by parent index and name
    entities: Vec<(u16, Name)>,
    /// Attribute is made by the entity index and a string that combines
    /// component name followed by their attributes spaced by a period,
    /// like so: `"Transform.translation.x"`
    attribute: Vec<(u16, Name)>,
    curves: Vec<CurveUntyped>,
}

// fn clip_default_warp() -> bool {
//     true
// }

impl Default for Clip {
    fn default() -> Self {
        Self {
            warp: true,
            duration: 0.0,
            // ? NOTE: Since the root has no parent in this context it points to a place outside the vec bounds
            entities: vec![(u16::MAX, Name::default())],
            attribute: vec![],
            curves: vec![],
        }
    }
}

impl Clip {
    /// Property to be animated must be in the following format `"path/to/named_entity@Transform.translation.x"`
    /// where the left side `@` defines a path to the entity to animate,
    /// while the right side the path to a property to animate starting from the component.
    pub fn add_animated_prop(&mut self, property_path: &str, mut curve: CurveUntyped) {
        // Clip an only have some amount of curves and entities
        // this limitation was added to save memory (but you can increase it if you want)
        assert!(
            (self.curves.len() as u16) <= u16::MAX,
            "curve limit reached"
        );

        let path =
            property_path.split_at(property_path.rfind('@').expect("property path missing @"));

        // TODO: Prepare for bevy_reflect
        let mut entity_created = false;
        let mut entity = 0; // Start search from root
        for name in path.0.split('/') {
            // Ignore the first '/' or '///'
            if name.is_empty() {
                continue;
            }

            if let Some(e) = self
                .entities
                .iter()
                .position(|(p, n)| (*p, n.as_str()) == (entity, name))
            {
                // Found entity
                entity = e as u16;
            } else {
                // Add entity
                let e = self.entities.len();
                self.entities.push((entity, Name::from_str(name)));
                entity_created = true;
                // Soft limit added to save memory, identical to the curve limit
                entity = u16::try_from(e).expect("entities limit reached");
            }
        }

        let insert_attr = Name::from_str(path.1.split_at(1).1);

        // If some entity was created it means this property is a new one
        if !entity_created {
            for (i, attr) in self.attribute.iter().enumerate() {
                if attr.0 != entity {
                    continue;
                }

                // NOTE: "...@Transform.translation" and "...@Transform.translation.x" can't live with each other
                // but since Properties aren't implemented for f32, Vec2, Vec3, Vec4 and Quat
                // (all relevant animated type values) the check rule could be simplified (a lot)
                //
                // The test case `test::unhandled_properties_implementation_for_types` will ensure that

                if insert_attr == attr.1 {
                    // Found a property are equal the one been inserted
                    // Replace curve, the property was already added, this is very important
                    // because it guarantees that each property will have unique access to some
                    // attribute during the update stages

                    let inserted_duration = curve.duration();
                    std::mem::swap(&mut self.curves[i], &mut curve);
                    self.update_duration(curve.duration(), inserted_duration);

                    return;
                }
            }
        }

        self.duration = self.duration.max(curve.duration());
        self.attribute.push((entity, insert_attr));
        self.curves.push(curve);
    }

    /// Number of animated properties in this clip
    #[inline(always)]
    pub fn len(&self) -> u16 {
        self.curves.len() as u16
    }

    /// Returns the property curve property path.
    ///
    /// The clip stores a property path in a specific way to improve search performance
    /// thus it needs to rebuilt the curve property path in the human readable format
    pub fn get_property_path(&self, index: u16) -> String {
        let (entity, name) = &self.attribute[index as usize];
        let mut path = format!("@{}", name.as_str());

        let mut first = true;
        let mut entity = *entity;
        while let Some((parent, name)) = self.entities.get(entity as usize) {
            if first {
                path = format!("{}{}", name.as_str(), path);
                first = false;
            } else {
                path = format!("{}/{}", name.as_str(), path);
            }
            entity = *parent;
        }

        path
    }

    /// Clip duration
    #[inline(always)]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Updates the clip duration based of the inserted and removed curve durations
    /// *WARNING* The curve must be already replaced before call this function
    fn update_duration(&mut self, removed_duration: f32, inserted_duration: f32) {
        if removed_duration == inserted_duration {
            // If precisely matches the inserted curve duration we don't need to update anything
        } else if float_cmp::approx_eq!(f32, removed_duration, self.duration, ulps = 2) {
            // At this point the duration for the removed curve is the same as self.duration
            // this mean that we have to compute the clip duration from scratch;
            //
            // NOTE: I opted for am approximated float comparison because it's better
            // to do all the this work than get a hard to debug glitch
            // TODO: Review approximated comparison

            self.duration = self
                .curves
                .iter()
                .map(|c| c.duration())
                .fold(0.0, |acc, x| acc.max(x));
        } else {
            // Faster clip duration update
            self.duration = self.duration.max(inserted_duration);
        }
    }
}

// TODO: Not found clip masks very useful, right now you can slone the cu
// /// Masks clip
// pub struct ClipMask {
//     /// Entity identification made by parent index and name
//     entities: Vec<(u16, String)>,
//     masked: Vec<u16>,
// }

// TODO: impl Serialize, Deserialize
#[derive(Debug, Clone)]
pub enum CurveUntyped {
    Float(Curve<f32>),
    Vec3(Curve<Vec3>),
    Vec4(Curve<Vec4>),
    Quat(Curve<Quat>),
    //Handle(Curve<HandleUntyped>),
    // TODO: Color(Curve<Color>),
}

macro_rules! untyped_fn {
    ($v:vis fn $name:ident ( &self, $( $arg:ident : $arg_ty:ty ,)* ) $(-> $ret:ty)* ) => {
        $v fn $name(&self, $( $arg : $arg_ty ),*) $(-> $ret)* {
            match self {
                CurveUntyped::Float(v) => v.$name($($arg,)*),
                CurveUntyped::Vec3(v) => v.$name($($arg,)*),
                CurveUntyped::Vec4(v) => v.$name($($arg,)*),
                CurveUntyped::Quat(v) => v.$name($($arg,)*),
            }
        }
    };
}

impl CurveUntyped {
    untyped_fn!(pub fn duration(&self,) -> f32);
    untyped_fn!(pub fn value_type(&self,) -> TypeId);
    //untyped_fn!(pub fn add_time_offset(&mut self, time: f32,) -> ());

    pub fn add_time_offset(&mut self, time: f32) {
        match self {
            CurveUntyped::Float(v) => v.add_offset_time(time),
            CurveUntyped::Vec3(v) => v.add_offset_time(time),
            CurveUntyped::Vec4(v) => v.add_offset_time(time),
            CurveUntyped::Quat(v) => v.add_offset_time(time),
        }
    }
}

// TODO: impl Serialize, Deserialize
#[derive(Default, Debug)]
pub struct Curve<T> {
    // TODO: Step / Linear / Spline variants
    samples: Vec<f32>,
    //tangents: Vec<(f32, f32)>,
    values: Vec<T>,
}

impl<T: Clone> Clone for Curve<T> {
    fn clone(&self) -> Self {
        Self {
            samples: self.samples.clone(),
            values: self.values.clone(),
        }
    }
}

impl<T> Curve<T>
where
    T: LerpValue + Clone + 'static,
{
    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        // Make sure both have the same length
        assert!(
            samples.len() == values.len(),
            "samples and values must have the same length"
        );

        assert!(samples.len() > 0, "empty curve");

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );
        Self { samples, values }
    }

    pub fn from_linear(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        Self {
            samples: if t1 >= t0 { vec![t0, t1] } else { vec![t1, t0] },
            values: vec![v0, v1],
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            samples: vec![0.0],
            values: vec![v],
        }
    }

    pub fn duration(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    /// Easer to use sampling method that don't have time restrictions or needs
    /// the keyframe index, but is more expensive always `O(n)`. Which means
    /// sampling takes longer to evaluate as much as time get closer to curve duration
    /// and it get worse with more keyframes.
    pub fn sample(&self, time: f32) -> T {
        // Index guessing gives a small search optimization
        let index = if time < self.duration() * 0.5 {
            0
        } else {
            self.samples.len() - 1
        };

        self.sample_indexed(index, time).1
    }

    /// Samples the curve starting from some keyframe index, this make the common case `O(1)`,
    /// use only when time advancing forwards
    pub fn sample_indexed(&self, mut index: usize, time: f32) -> (usize, T) {
        // Adjust for the current keyframe index
        let last_index = self.samples.len() - 1;

        index = index.max(0).min(last_index);
        if self.samples[index] < time {
            // Forward search
            loop {
                if index == last_index {
                    return (last_index, self.values.last().unwrap().clone());
                }
                index += 1;

                if self.samples[index] >= time {
                    break;
                }
            }
        } else {
            // Backward search
            loop {
                if index == 0 {
                    return (0, self.values.last().unwrap().clone());
                }

                let i = index - 1;
                if self.samples[i] <= time {
                    break;
                }

                index = i;
            }
        }

        // Lerp the value
        let i = index - 1;
        let previous_time = self.samples[i];
        let t = (time - previous_time) / (self.samples[index] - previous_time);
        debug_assert!(t >= 0.0 && t <= 1.0, "t = {} but should be normalized", t); // Checks if it's required to normalize t
        let value = T::lerp(&self.values[i], &self.values[index], t);

        (index, value)
    }

    pub fn add_offset_time(&mut self, time_offset: f32) {
        self.samples.iter_mut().for_each(|t| *t += time_offset);
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    //assert!(samples.len() > 1, "curve can't be empty");
    // }

    pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
        self.samples.iter().copied().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
        self.samples.iter().copied().zip(self.values.iter_mut())
    }

    pub fn value_type(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

#[derive(Debug, Clone, Properties)]
pub struct Layer {
    pub weight: f32,
    pub clip: u16,
    pub time: f32,
    pub time_scale: f32,
    keyframe: Vec<usize>,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            weight: 1.0,
            clip: 0,
            time: 0.0,
            time_scale: 1.0,
            keyframe: vec![],
        }
    }
}

#[derive(Debug, Properties)]
pub struct Animator {
    clips: Vec<Handle<Clip>>,
    #[property(ignore)]
    bind_clips: Vec<Option<ClipBind>>,
    // masks: Vec<Handle<ClipMask>>,
    // #[property(ignore)]
    // bind_masks: Vec<ClipMaskBind>,
    /// Used to blend all the active layers, durning the frame update each pointers
    /// can only be set once, by the second time then need to perform blended with the next layer
    ///
    // ? NOTE: We may need to change this approach when blending more than 2 layers
    // ? NOTE: It takes more memory but it's faster than having a local resource with a big chunk of data or even allocating at every frame
    #[property(ignore)]
    visited: fnv::FnvHashSet<Ptr>,
    pub time_scale: f32,
    pub layers: Vec<Layer>,
}

// TODO: AnimatorStateMachine + Transitions to simplify layer blending but in another file

impl Default for Animator {
    fn default() -> Self {
        Self {
            clips: vec![],
            bind_clips: vec![],
            visited: fnv::FnvHashSet::default(),
            time_scale: 1.0,
            layers: vec![],
        }
    }
}

impl Animator {
    pub fn add_clip(&mut self, clip: Handle<Clip>) -> u16 {
        if let Some(i) = self.clips.iter().position(|c| *c == clip) {
            i as u16
        } else {
            // TODO: assert too many clips ...
            let i = self.clips.len();
            self.clips.push(clip);
            i as u16
        }
    }

    pub fn add_layer(&mut self, clip: Handle<Clip>, weight: f32) -> u16 {
        let clip = self.add_clip(clip);
        let layer_index = self.layers.len();
        self.layers.push(Layer {
            clip,
            weight,
            ..Default::default()
        });
        layer_index as u16
    }

    pub fn clips_len(&self) -> u16 {
        self.clips.len() as u16
    }
}

// TODO: Bind asset properties (edge case), notice that this feature will be on hold
// until the atelier assets get through. I will probably just store pointers offsets
// because require less conditions to be safe and is not too much that expensive to store
// direct pointers

#[derive(Debug)]
struct ClipBindMetadataInner {
    parent: Option<Entity>,
    entity: Entity,
    location: Location,
    component: TypeId,
}

/// Metadata is used to check the bind validity
#[derive(Debug)]
struct ClipBindMetadata {
    entity_index: u16,
    inner: Option<ClipBindMetadataInner>,
    range: (u16, u16),
}

#[derive(Debug)]
struct ClipBind {
    metadata: Vec<ClipBindMetadata>,
    curves: Vec<u16>,
    attributes: Vec<Ptr>,
}

// #[derive(Debug)]
// struct ClipMaskBind {
//     metadata: Vec<(Option<Entity>, Entity)>
//     allowed_entities: HashMap<Entity>,
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*mut u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

/// Marker type for missing component
struct MissingComponentMarker;

#[derive(Default)]
struct ClipResourceProviderState {
    clip_event_reader: EventReader<AssetEvent<Clip>>,
}

// TODO: Refactor in hope to reuse more code (maybe just create more functions)
#[tracing::instrument(skip(world, resources))]
pub fn animator_binding_system(world: &mut World, resources: &mut Resources) {
    let system_id = SystemId(0); // ? NOTE: shouldn't be required to match the system id

    // TODO: These local resources seams to be quite slow, maybe I need a random system id?
    // Fetch resources
    let mut state = {
        let state = resources.get_local_mut::<ClipResourceProviderState>(system_id);
        if state.is_none() {
            std::mem::drop(state); // ? NOTE: Makes the borrow checker happy
            resources.insert_local(system_id, ClipResourceProviderState::default());
            resources
                .get_local_mut::<ClipResourceProviderState>(system_id)
                .unwrap()
        } else {
            state.unwrap()
        }
    };

    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let type_registry_read_guard = type_registry.component.read();

    let clips = resources.get::<Assets<Clip>>().unwrap();
    let clip_events = resources.get::<Events<AssetEvent<Clip>>>().unwrap();

    // Builds query
    // SAFETY: This is the only query with mutable reference the Animator component
    let animator_query = unsafe { world.query_unchecked::<(Entity, &mut Animator), ()>() };

    // ? NOTE: Changing a clip on fly is supported, but very expensive so use with caution
    // Query all clips that changed and remove their binds from the animator
    let mut modified = HashSet::new();
    for event in state.clip_event_reader.iter(&clip_events) {
        match event {
            AssetEvent::Removed { handle } => modified.insert(handle),
            AssetEvent::Modified { handle } => modified.insert(handle),
            _ => false,
        };
    }

    for (root, mut animator) in animator_query {
        let animator = &mut *animator; // Make sure the borrow checker is happy

        // Make room for binds or deallocate unused
        animator
            .bind_clips
            .resize_with(animator.clips.len(), || None);

        // Used for entity translation
        let mut entities_table_cache = vec![];

        for (clip_handle, bind) in animator.clips.iter().zip(animator.bind_clips.iter_mut()) {
            // Invalidate binds for clips that where modified
            if modified.contains(clip_handle) {
                *bind = None;
            }

            if let Some(clip) = clips.get(clip_handle) {
                // Prepare the entities table cache
                entities_table_cache.clear();
                entities_table_cache.resize(clip.entities.len(), None);
                // Assign the root entity as the first element
                entities_table_cache[0] = Some(root);

                if bind.is_none() {
                    // Build binds from scratch
                    // Allocate the minium enough of memory on right at the begin
                    let mut b = ClipBind {
                        metadata: Vec::with_capacity(clip.entities.len()),
                        curves: Vec::with_capacity(clip.attribute.len()),
                        attributes: Vec::with_capacity(clip.attribute.len()),
                    };

                    let mut range = (0, 0);
                    let mut prev_partial_info = None;
                    let mut prev_component = TypeId::of::<MissingComponentMarker>();
                    let mut prev_component_short_name = "";

                    for (curve_index, (entity_index, attr_path)) in
                        clip.attribute.iter().enumerate()
                    {
                        let mut commit = false;
                        let mut partial_info = None;
                        let mut component = TypeId::of::<MissingComponentMarker>();

                        // Query component by name
                        let mut path = attr_path.split('.');
                        let component_short_name =
                            path.next().expect("missing component short name");

                        if let Some(entity) = find_entity(
                            *entity_index,
                            &clip.entities,
                            &mut entities_table_cache,
                            &world,
                        ) {
                            let location = world.get_entity_location(entity).unwrap();

                            // Commit is needed
                            partial_info = Some((entity, location));
                            if prev_partial_info != partial_info {
                                commit = true;
                            }

                            if let Some(component_reg) =
                                type_registry_read_guard.get_with_short_name(component_short_name)
                            {
                                component = component_reg.ty;

                                let root_props = component_reg.get_component_properties(
                                    &world.archetypes[location.archetype as usize],
                                    location.index,
                                );

                                b.curves.push(curve_index as u16);
                                b.attributes.push(find_property_ptr(
                                    path,
                                    root_props,
                                    clip.curves[curve_index].value_type(),
                                ));

                                // Group by component type
                                commit |= prev_component != component;
                            } else {
                                // Missing component
                                b.curves.push(curve_index as u16);
                                b.attributes.push(Ptr(null_mut()));

                                // Group by component name
                                commit |= prev_component_short_name != component_short_name;
                            }
                        } else {
                            // Missing entity
                            commit = prev_partial_info != partial_info;
                            commit |= prev_component_short_name != component_short_name;

                            b.curves.push(curve_index as u16);
                            b.attributes.push(Ptr(null_mut()));
                        }

                        if commit {
                            // Close range
                            range.1 = b.curves.len() as u16 - 1;

                            // Commit meta
                            b.metadata.push(ClipBindMetadata {
                                entity_index: *entity_index,
                                inner: prev_partial_info.map(|(entity, location)| {
                                    ClipBindMetadataInner {
                                        // An entity will always have parent unless is the root entity
                                        parent: entities_table_cache
                                            .get(clip.entities[*entity_index as usize].0 as usize)
                                            .copied()
                                            .flatten(),
                                        entity,
                                        location,
                                        component,
                                    }
                                }),
                                range,
                            });

                            // Jump to the next range
                            prev_partial_info = partial_info;
                            prev_component = component;
                            prev_component_short_name = component_short_name;
                            range.0 = range.1;
                            range.1 += 1;
                        }
                    }

                    *bind = Some(b);

                    // No need to continue because binds are fresh
                    continue;
                }

                // Check bind state and update the necessary parts
                let bind = bind.as_mut().unwrap();
                for meta in bind.metadata.iter_mut() {
                    if meta.inner.is_none() {
                        // TODO: Handle missing entity
                        continue;
                    }

                    let inner = meta.inner.as_mut().unwrap();

                    // Handle `Parent` changed
                    if let Some(parent) = inner.parent {
                        // Not root
                        if let Ok(Parent(new_parent)) =
                            world.query_one_filtered::<&Parent, Changed<Parent>>(inner.entity)
                        {
                            if parent != *new_parent {
                                // TODO: Look again for the entity

                                // Invalidate bind ...
                                meta.inner = None;
                                for i in (meta.range.0)..(meta.range.1) {
                                    bind.attributes[i as usize] = Ptr(null_mut());
                                }
                                continue;
                            }
                        }
                    }

                    let mut loc = world.get_entity_location(inner.entity);
                    if loc.is_none() {
                        // Missing entity
                        if let Some(found_entity) = find_entity(
                            meta.entity_index,
                            &clip.entities,
                            &mut entities_table_cache,
                            &world,
                        ) {
                            // Found new entity
                            inner.entity = found_entity;
                            // Fetch it's location
                            loc = world.get_entity_location(found_entity);
                        } else {
                            // Entity wasn't found, invalidate all the bind attributes
                            meta.inner = None; // Clear inner data
                            for i in (meta.range.0)..(meta.range.1) {
                                bind.attributes[i as usize] = Ptr(null_mut());
                            }
                            continue;
                        }
                    }

                    let loc = loc.unwrap();
                    if inner.location == loc {
                        // Nothing left todo
                        continue;
                    }
                    if inner.component == TypeId::of::<MissingComponentMarker>() {
                        // TODO: Find component by name
                        continue;
                    }

                    // Entity archetype changed
                    if let Some(component_reg) = type_registry_read_guard.get(&inner.component) {
                        let root_props = component_reg.get_component_properties(
                            &world.archetypes[loc.archetype as usize],
                            loc.index,
                        );

                        // Update pointers to reflect the new entity memory location
                        for i in (meta.range.0)..(meta.range.1) {
                            let i = i as usize;
                            let curve_index = bind.curves[i] as usize;
                            let mut path = clip.attribute[curve_index].1.split('.');
                            path.next(); // Skip component

                            let ptr = find_property_ptr(
                                path,
                                root_props,
                                clip.curves[curve_index].value_type(),
                            );
                            debug_assert!(bind.attributes[i] != ptr, "pointers can't be the same");
                            bind.attributes[i] = ptr;
                        }
                    } else {
                        // Component is missing
                        for i in (meta.range.0)..(meta.range.1) {
                            bind.attributes[i as usize] = Ptr(null_mut());
                        }
                        inner.component = TypeId::of::<MissingComponentMarker>();
                    }

                    // Update bind location
                    inner.location = loc;
                }
            }
        }
    }
}

fn find_entity(
    entity_index: u16,
    entities_ids: &Vec<(u16, Name)>,
    entities_table_cache: &mut Vec<Option<Entity>>,
    world: &World,
) -> Option<Entity> {
    if let Some(entity) = &entities_table_cache[entity_index as usize] {
        Some(*entity)
    } else {
        let (parent_index, entity_name) = &entities_ids[entity_index as usize];

        // Use recursion to find the entity parent
        find_entity(*parent_index, entities_ids, entities_table_cache, world).and_then(
            |parent_entity| {
                if let Ok(children) = world.get::<Children>(parent_entity) {
                    children
                        .iter()
                        .find(|entity| {
                            if let Ok(name) = world.get::<Name>(**entity) {
                                if name == entity_name {
                                    // Update cache
                                    entities_table_cache[entity_index as usize] = Some(**entity);
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        })
                        .copied()
                } else {
                    None
                }
            },
        )
    }
}

fn find_property_at_path<'a, P: Iterator<Item = &'a str>>(
    path: P,
    root_props: &dyn Properties,
    type_id: TypeId,
) -> Option<&dyn Property> {
    let mut target = None;
    let mut current = Some(root_props);
    for property_name in path {
        if let Some(property) = current.map(|p| p.prop(property_name)).flatten() {
            // NOTE: Vec, HashMap and other types can't be animated
            // and needed to be filtered out
            if property.property_type() != PropertyType::Value {
                target = None;
                break;
            }

            target = Some(property);
            current = property.as_properties();
        } else {
            // Failed to find property
            target = None;
            break;
        }
    }

    target.and_then(|prop| {
        if prop.any().type_id() == type_id {
            Some(prop)
        } else {
            None
        }
    })
}

#[inline(always)]
fn find_property_ptr<'a, P: Iterator<Item = &'a str>>(
    path: P,
    root_props: &dyn Properties,
    type_id: TypeId,
) -> Ptr {
    Ptr(find_property_at_path(path, root_props, type_id)
        .map(|prop| {
            let ptr = prop as *const _ as *mut u8;
            // Perform an extra assertion to make sure this pointer is inside the component
            // memory bounds, so no dangle pointers can be created
            {
                let root = root_props as *const _ as *mut u8;
                let size = std::mem::size_of_val(root_props) as isize;
                // With the new bevy_reflect we can ref properties inside vectors and other things
                if unsafe { root.offset_from(ptr).abs() } >= size {
                    // TODO: log also the full property path
                    tracing::warn!("property outside component");
                    return null_mut();
                }
            }
            ptr
        })
        .unwrap_or(null_mut()))
}

#[tracing::instrument(skip(world, resources))]
pub fn animator_update_system(world: &mut World, resources: &mut Resources) {
    /// Small value
    const SMOL: f32 = 1e-8;

    let time = resources.get::<Time>().unwrap();
    let clips = resources.get::<Assets<Clip>>().unwrap();

    // SAFETY: This is the only query with mutable reference the Animator component
    let animators_query = unsafe { world.query_unchecked::<(&mut Animator,), ()>() };
    let delta_time = time.delta_seconds;

    // ? NOTE: Keep in mind that one hierarchy tree could have many `Animator`s thus
    // ? we can't parallelize over the animators easily without creating a race condition
    for (mut animator,) in animators_query {
        let animator = &mut *animator;

        // Time scales by component
        let delta_time = delta_time * animator.time_scale;

        let w_total = animator
            .layers
            .iter()
            .fold(0.0, |w, layer| w + layer.weight);

        let norm = 1.0 / w_total;

        // Normalize all states weights
        for layer in &mut animator.layers {
            layer.weight *= norm;
        }

        animator.visited.clear();
        for layer in &mut animator.layers {
            let w = layer.weight;
            if w < SMOL {
                continue;
            }

            let clip_index = layer.clip;
            if let Some(clip) = animator
                .clips
                .get(clip_index as usize)
                .map(|clip_handle| clips.get(clip_handle))
                .flatten()
            {
                // TODO: Add layer mask

                if let Some(bind) = &animator.bind_clips[clip_index as usize] {
                    let curves_count = clip.curves.len();

                    // Ensure capacity for cached keyframe index vec
                    if layer.keyframe.len() != curves_count {
                        layer.keyframe.clear();
                        layer
                            .keyframe
                            .resize_with(curves_count, || Default::default());
                    }

                    // Update time
                    let mut time = layer.time + delta_time * layer.time_scale;

                    // TODO: I notice some jitter during playback

                    // Warp mode
                    if clip.warp {
                        // Warp Around
                        if time > clip.duration {
                            time = (time / clip.duration).fract() * clip.duration;
                            // Reset all keyframes cached indexes
                            layer
                                .keyframe
                                .iter_mut()
                                .for_each(|x| *x = Default::default())
                        }
                    } else {
                        // Hold
                        time = time.min(clip.duration);
                    }

                    // Advance state time
                    layer.time = time;

                    // TODO: can be done in bached_parallel if is visited warped inside a mutex
                    for ((ptr, keyframe), curve_index) in bind
                        .attributes
                        .iter()
                        .zip(layer.keyframe.iter_mut())
                        .zip(bind.curves.iter())
                    {
                        // Skip invalid properties
                        if *ptr == Ptr(null_mut()) {
                            continue;
                        }

                        let curve = &clip.curves[*curve_index as usize];

                        let viz = if animator.visited.contains(&ptr) {
                            true
                        } else {
                            animator.visited.insert(*ptr);
                            false
                        };

                        // SAFETY: The `animator_binding_system` is responsible to invalidate and/or update
                        // any ptr that no longer pointers to the right component for the right entity;
                        // Also it will only allows to ptr to attributes by value inside the component this means
                        // no dangle pointers

                        match curve {
                            CurveUntyped::Float(v) => {
                                let (k, v) = v.sample_indexed(*keyframe, time);
                                *keyframe = k;

                                unsafe {
                                    let ptr = ptr.0 as *mut f32;
                                    if viz {
                                        *ptr = LerpValue::lerp(&*ptr, &v, w);
                                    } else {
                                        *ptr = v;
                                    }
                                }
                            }
                            CurveUntyped::Vec3(v) => {
                                let (k, v) = v.sample_indexed(*keyframe, time);
                                *keyframe = k;

                                unsafe {
                                    let ptr = ptr.0 as *mut Vec3;
                                    if viz {
                                        *ptr = LerpValue::lerp(&*ptr, &v, w);
                                    } else {
                                        *ptr = v;
                                    }
                                }
                            }
                            CurveUntyped::Vec4(c) => {
                                let (k, v) = c.sample_indexed(*keyframe, time);
                                *keyframe = k;

                                unsafe {
                                    let ptr = ptr.0 as *mut Vec4;
                                    if viz {
                                        *ptr = LerpValue::lerp(&*ptr, &v, w);
                                    } else {
                                        *ptr = v;
                                    }
                                }
                            }
                            CurveUntyped::Quat(v) => {
                                let (k, v) = v.sample_indexed(*keyframe, time);
                                *keyframe = k;

                                unsafe {
                                    let ptr = ptr.0 as *mut Quat;
                                    if viz {
                                        // ? NOTE: Always nlerp, because blending the same clips in different order must yield the same results
                                        // ! NOTE: This now one of (if not) the most expensive thing been executed in the animator_update_system
                                        // ! blending more layers will increase the cost, change this behavior will result in change of the final motion
                                        // ! in unexpected ways (maybe not by much but still)
                                        *ptr = LerpValue::lerp(&*ptr, &v, w);
                                    } else {
                                        *ptr = v;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // TODO: Make sure to update the components states to trigger 'Mutated'
    }
}

// pub struct ClipLoader;

// impl AssetLoader for ClipLoader {
//     fn load<'a>(
//         &'a self,
//         bytes: &'a [u8],
//         load_context: &'a mut bevy::asset::LoadContext,
//     ) -> bevy::utils::BoxedFuture<'a, Result<()>> {
//         Box::pin(async move {
//             let clip: Clip = ron::de::from_bytes(bytes)?;
//             load_context.set_default_asset(LoadedAsset::new(clip));
//             Ok(())
//         })
//     }

//     fn extensions(&self) -> &[&str] {
//         &["anim"]
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_evaluation() {
        let curve = Curve::new(
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
        );
        assert_eq!(curve.sample(0.5), 1.0);

        let mut i0 = 0;
        let mut e0 = 0.0;
        for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
            let v = *v;
            let (i1, e1) = curve.sample_indexed(i0, v);
            assert_eq!(e1, 2.0 * v);
            if e1 > e0 {
                assert!(i1 >= i0);
            } else {
                assert!(i1 <= i0);
            }
            e0 = e1;
            i0 = i1;
        }
    }

    #[test]
    #[should_panic]
    fn curve_bad_length() {
        let _ = Curve::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = Curve::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn create_clip() {
        let mut clip = Clip::default();
        let curve = Curve::from_linear(0.0, 1.0, 0.0, 1.0);
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(prop, CurveUntyped::Float(curve));
        assert_eq!(clip.get_property_path(0), prop);
    }

    #[test]
    fn clip_replace_property() {
        // NOTE: This test is very important because it guarantees that each property have unique
        // access to some attribute during the update stages
        let mut clip = Clip::default();
        let prop = "/Root/Ball@Sphere.radius";
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.y",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        assert_eq!(clip.duration(), 1.0);
        clip.add_animated_prop(
            prop,
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.1, 2.0)),
        );
        assert_eq!(clip.len(), 2);
        assert_eq!(clip.get_property_path(1), prop);
        assert_eq!(clip.duration(), 1.2);
    }

    #[test]
    fn clip_fine_grain_properties() {
        let mut clip = Clip::default();
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.y",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.0, 0.0, 1.0)),
        );
        assert_eq!(clip.duration(), 1.0);
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.x",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.0, 2.0)),
        );
        assert_eq!(clip.len(), 2);
        assert_eq!(clip.duration(), 1.2);
    }

    #[test]
    #[should_panic]
    fn clip_nested_properties() {
        // Maybe required by `bevy_reflect` this guarantees are necessary
        // because the way we execute
        let mut clip = Clip::default();
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate",
            CurveUntyped::Vec3(Curve::from_linear(0.0, 1.0, Vec3::zero(), Vec3::unit_y())),
        );
        clip.add_animated_prop(
            "/Root/Ball@Transform.translate.x",
            CurveUntyped::Float(Curve::from_linear(0.0, 1.2, 0.0, 2.0)),
        );
    }

    #[test]
    fn unhandled_properties_implementation_for_types() {
        // This is take as granted to simplify the function `Clip::add_animated_prop`
        // you can give a look into it to see more about their implications

        macro_rules! check_for_properties {
            ($t:ty) => {{
                let v = <$t>::default();
                let p: &dyn Property = &v;
                assert!(
                    p.as_properties().is_none(),
                    "unhandled impl `Properties` for {}",
                    stringify!($t)
                );
            }};
        }

        check_for_properties!(f32);
        check_for_properties!(Vec2);
        check_for_properties!(Vec3);
        //check_for_properties!(Vec4); // TODO: Vec4 should implement at least Property
        check_for_properties!(Quat);
        //check_for_properties!(Color); // TODO: sRGB Color
    }

    // ? NOTE: Don't remove this test, right now we can skip this test
    // ? because all the animated type values (Vec2, Vec3 ...) don't
    // ? implement the `Properties` type and thus none of their inner
    // ? can be accessed
    // #[test]
    // #[should_panic]
    // fn clip_add_nested_property() {
    //     let mut clip = Clip::default();
    //     let position = Curve::from_linear(0.0, 1.0, Vec3::zero(), Vec3::unit_y());
    //     let y = Curve::from_linear(0.0, 1.0, 0.0, 1.0);
    //     clip.add_animated_prop(
    //         "Root/Ball@Transform.translation",
    //         CurveUntyped::Vec3(position),
    //     );
    //     // NOTE: Right now "Transform.translation.y" can't be accessed because Vec3 doesn't impl Properties
    //     clip.add_animated_prop("Root/Ball@Transform.translation.y", CurveUntyped::Float(y));
    // }

    // TODO: Add tests for hierarch change (creation and deletion)
    // TODO: Add tests for entity archetype change
    // TODO: Add tests for entities with same Name
    // TODO: Add tests for mutated events
    // TODO: Add tests for animated handle (impl missing)
    // TODO: Add tests for animated asset properties (impl missing)
    // TODO: Add tests for animated components should trigger 'Mutated' queries filters (impl missing)

    struct AnimtorTestBench {
        app: bevy_app::App,
        //binding_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>>,
        update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>>,
    }

    impl AnimtorTestBench {
        fn new() -> Self {
            let mut app_builder = bevy_app::App::build();
            app_builder
                .add_plugin(bevy_type_registry::TypeRegistryPlugin::default())
                .add_plugin(bevy_core::CorePlugin::default())
                .add_plugin(bevy_app::ScheduleRunnerPlugin::default())
                .add_plugin(bevy_asset::AssetPlugin)
                .add_plugin(bevy_transform::TransformPlugin)
                .add_plugin(crate::AnimationPlugin);

            let mut world = World::new();
            let mut world_builder = world.build();
            let base = (
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            );

            // Create animator and assign some clips
            let mut animator = Animator::default();
            {
                let mut clip_a = Clip::default();
                clip_a.add_animated_prop(
                    "@Transform.translation",
                    CurveUntyped::Vec3(Curve::from_linear(
                        0.0,
                        1.0,
                        Vec3::unit_x(),
                        -Vec3::unit_x(),
                    )),
                );
                let rot = CurveUntyped::Quat(Curve::from_constant(Quat::identity()));
                clip_a.add_animated_prop("@Transform.rotation", rot.clone());
                clip_a.add_animated_prop("/Hode1@Transform.rotation", rot.clone());
                clip_a.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

                let mut clip_b = Clip::default();
                clip_b.add_animated_prop(
                    "@Transform.translation",
                    CurveUntyped::Vec3(Curve::from_constant(Vec3::zero())),
                );
                let rot = CurveUntyped::Quat(Curve::from_linear(
                    0.0,
                    1.0,
                    Quat::from_axis_angle(Vec3::unit_z(), 0.1),
                    Quat::from_axis_angle(Vec3::unit_z(), -0.1),
                ));
                clip_b.add_animated_prop("@Transform.rotation", rot.clone());
                clip_b.add_animated_prop("/Hode1@Transform.rotation", rot.clone());
                clip_b.add_animated_prop("/Node1/Node2@Transform.rotation", rot);

                let mut clips = app_builder
                    .resources_mut()
                    .get_mut::<Assets<Clip>>()
                    .unwrap();
                let clip_a = clips.add(clip_a);
                let clip_b = clips.add(clip_b);

                animator.add_layer(clip_a, 0.5);
                animator.add_layer(clip_b, 0.5);
            }

            world_builder
                .spawn(base.clone())
                .with(Name::from_str("Root"))
                .with(animator)
                .with_children(|world_builder| {
                    world_builder
                        .spawn(base.clone())
                        .with(Name::from_str("Node1"))
                        .with_children(|world_builder| {
                            world_builder
                                .spawn(base.clone())
                                .with(Name::from_str("Node2"))
                                .with_children(|world_builder| {
                                    world_builder
                                        .spawn(base.clone())
                                        .with(Name::from_str("Node3"));
                                });
                        });
                });

            app_builder.set_world(world);

            // let mut parent_update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
            //     Box::new(bevy_transform::hierarchy::parent_update_system.system());

            let mut binding_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
                Box::new(crate::animator_binding_system.system());

            let update_system: Box<dyn bevy_ecs::System<Input = (), Output = ()>> =
                Box::new(crate::animator_update_system.system());

            let mut app = app_builder.app;

            //parent_update_system.run((), &mut app.world, &mut app.resources);
            binding_system.run_thread_local(&mut app.world, &mut app.resources);

            Self { app, update_system }
        }

        fn update(&mut self, ticks: usize) {
            for _ in 0..ticks {
                // Time tick
                {
                    let mut time = self.app.resources.get_mut::<Time>().unwrap();
                    time.delta_seconds += 0.016;
                    time.delta_seconds_f64 += 0.016;
                }

                self.update_system
                    .run_thread_local(&mut self.app.world, &mut self.app.resources);

                //dbg!(app.world.get::<Transform>(root_entity).unwrap());
            }
        }
    }

    #[test]
    fn test_bench_update() {
        // ? NOTE: Mimics a basic system update behavior good for pref since criterion will pollute the
        // ? annotations with many expensive instructions
        let mut a = AnimtorTestBench::new();
        a.update(1_000_000);
    }

    // TODO: 5 layers or more ...
}
