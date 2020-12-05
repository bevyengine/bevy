use anyhow::Result;
use bevy_app::prelude::{EventReader, Events};
use bevy_asset::{AssetEvent, Assets, Handle /*, HandleUntyped*/};
use bevy_core::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::{Location, SystemId};
use bevy_math::prelude::*;
use bevy_property::{Properties, Property, PropertyType};
use bevy_transform::prelude::*;
use bevy_type_registry::TypeRegistry;
//use serde::{Deserialize, Serialize};
use std::any::TypeId;
use std::hash::Hash;
use std::ptr::null_mut;

use super::clip::Clip;
use super::curve::CurveUntyped;
use super::lerping::LerpValue;

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
struct ClipBindEntity {
    parent: Option<Entity>,
    entity: Entity,
    location: Location,
}

#[derive(Debug)]
struct ClipBind {
    entities: Vec<Option<ClipBindEntity>>,
    /// True only when all the attributes are binded
    binded: bool,
    /// Attributes pointers
    pointers: Vec<Ptr>,
}

// #[derive(Debug)]
// struct ClipMaskBind {
//     allowed_entities: HashMap<Entity>,
// }

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Ptr(*mut u8);

// SAFETY: Store pointers to each attribute to be updated, a clip can't have two pointers
// with the same value. Each clip per Animator will be updated sequentially
unsafe impl Send for Ptr {}
unsafe impl Sync for Ptr {}

// /// Marker type for missing component
// struct MissingComponentMarker;

/// State info for the system `animator_binding_system`
#[derive(Default)]
struct BindingState {
    clips_event_reader: EventReader<AssetEvent<Clip>>,
    clips_modified: fnv::FnvHashSet<Handle<Clip>>,
}

/// Creates a new vector with a given size filled with the values returned by the given default function
#[inline(always)]
fn vec_filled<T, F: Fn() -> T>(size: usize, default: F) -> Vec<T> {
    let mut v = vec![];
    v.resize_with(size, default);
    v
}

#[tracing::instrument(skip(world, resources))]
pub fn animator_binding_system(world: &mut World, resources: &mut Resources) {
    let system_id = SystemId(0); // ? NOTE: shouldn't be required to match the system id

    // ! FIXME: These local resources seams to be quite slow, maybe I need a random system id?
    // Fetch resources
    let mut state = {
        let state = resources.get_local_mut::<BindingState>(system_id);
        match state {
            Some(state) => state,
            None => {
                std::mem::drop(state); // ? NOTE: Makes the borrow checker happy
                resources.insert_local(system_id, BindingState::default());
                resources.get_local_mut::<BindingState>(system_id).unwrap()
            }
        }
    };

    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let mut type_registry_read_guard = None;

    let clips = resources.get::<Assets<Clip>>().unwrap();
    let clip_events = resources.get::<Events<AssetEvent<Clip>>>().unwrap();

    // Builds query
    // SAFETY: This is the only query with mutable reference the Animator component
    let animator_query = unsafe { world.query_unchecked::<(Entity, &mut Animator), ()>() };

    // ? NOTE: Changing a clip on fly is supported, but very expensive so use with caution
    // Query all clips that changed and remove their binds from the animator
    state.clips_modified.clear();
    for event in state.clips_event_reader.iter(&clip_events) {
        match event {
            AssetEvent::Removed { handle } => state.clips_modified.insert(handle.clone()),
            AssetEvent::Modified { handle } => state.clips_modified.insert(handle.clone()),
            _ => false,
        };
    }

    let mut entities_parent_changed = vec![];

    // Used for entity translation
    let mut entities_table_cache = vec![];

    for (root, mut animator) in animator_query {
        let animator = &mut *animator; // Make sure the borrow checker is happy

        // Make room for binds or deallocate unused
        animator
            .bind_clips
            .resize_with(animator.clips.len(), || None);

        for (clip_handle, bind) in animator.clips.iter().zip(animator.bind_clips.iter_mut()) {
            // Invalidate binds for clips that where modified
            if state.clips_modified.contains(clip_handle) {
                *bind = None;
            }

            let clip = clips.get(clip_handle);
            if clip.is_none() {
                // Clip missing, invalidate binds if any
                *bind = None;
                continue;
            }

            // Unwraps the clip
            let clip = clip.unwrap();

            // Prepare the entities table cache
            entities_table_cache.clear();
            entities_table_cache.resize(clip.hierarchy.len(), None);
            // Assign the root entity as the first element
            entities_table_cache[0] = Some(root);

            let bind = if let Some(bind) = bind.as_mut() {
                // Invalidate previously binded attributes

                // Borrow checker sugar coat
                let binded = &mut bind.binded;
                let pointers = &mut bind.pointers;
                let entities = &mut bind.entities;

                // NOTE: Must be empty in this next function, if not it means
                // something went wrong in the previous iteration
                debug_assert!(
                    entities_parent_changed.len() == 0,
                    "parent changes have not been properly handled"
                );

                entities
                    .iter_mut()
                    .enumerate()
                    // Find entities that changed
                    .filter(|(entity_index, entry)| {
                        if entry.is_none() {
                            // Already invalid
                            return false;
                        }
                        let inner = entry.as_ref().unwrap();

                        // Not root
                        if *entity_index > 0 {
                            // TODO: Check the entity name
                            if let Ok(Parent(parent)) = world.query_one::<&Parent>(inner.entity) {
                                // Parent changed or removed
                                if inner.parent != Some(*parent) {
                                    entities_parent_changed.push(*entity_index as u16);
                                    return true;
                                }
                            } else {
                                // Parent removed
                                entities_parent_changed.push(*entity_index as u16);
                                return true;
                            }
                        }

                        if let Some(loc) = world.get_entity_location(inner.entity) {
                            if inner.location != loc {
                                // Entity archetype changed
                                return true;
                            }
                        } else {
                            // Entity deleted
                            entities_parent_changed.push(*entity_index as u16);
                            return true;
                        }

                        // Entity didn't change
                        false
                    })
                    // Invalidate all entities that changed
                    .for_each(|(entity_index, entry)| {
                        *entry = None;
                        // ? NOTE: Will only run if the bind was invalidated in this frame
                        for (descriptor, attr) in clip.properties.iter().zip(pointers.iter_mut()) {
                            if descriptor.0 as usize == entity_index {
                                *attr = Ptr(null_mut());
                            }
                        }
                        *binded = false;
                    });

                // Handle entities that had theirs parents changed
                // ? NOTE: Heavy code path triggered when an animated hierarchy changes
                while let Some(entity_index) = entities_parent_changed.pop() {
                    clip.hierarchy
                        .depth_first(entity_index, &mut |entity_index, _| {
                            let entry = &mut entities[entity_index as usize];

                            if entry.is_some() {
                                // Yet not invalidated
                                for (descriptor, attr) in
                                    clip.properties.iter().zip(pointers.iter_mut())
                                {
                                    if descriptor.0 == entity_index {
                                        *attr = Ptr(null_mut());
                                    }
                                }

                                *entry = None;
                            }

                            *binded = false;
                        });
                }

                bind
            } else {
                // Create empty binds
                *bind = Some(ClipBind {
                    entities: vec_filled(clip.hierarchy.len(), || None),
                    binded: false,
                    pointers: vec_filled(clip.properties.len(), || Ptr(null_mut())),
                });

                bind.as_mut().unwrap()
            };

            // All attributes are binded
            if bind.binded {
                continue;
            }

            // Will be cleared if any attribute fail to bind
            bind.binded = true;

            // Bind attributes
            for (attr_index, (attr, descriptor)) in bind
                .pointers
                .iter_mut()
                .zip(clip.properties.iter())
                .enumerate()
            {
                let (entity_index, attr_path) = descriptor;

                if *attr != Ptr(null_mut()) {
                    // Already binded
                    continue;
                }

                // ! NOTE: Unbinded attributes will trigger this code path every frame,
                // ! so missing attributes will make a dent in the performance,

                // Query component by name
                // TODO: Log error instead of panic ...
                let mut path = attr_path.split('.');
                let component_short_name = path.next().expect("missing component short name");

                if let Some(entity) = clip.hierarchy.find_entity_in_world(
                    *entity_index,
                    &mut entities_table_cache,
                    &world,
                ) {
                    let location = world.get_entity_location(entity).unwrap();

                    bind.entities[*entity_index as usize].get_or_insert_with(|| {
                        ClipBindEntity {
                            // If the entity was found the parent was as well (unless is the root which will always be None)
                            parent: entities_table_cache
                                .get(clip.hierarchy.get_entity(*entity_index).0 as usize)
                                .copied()
                                .flatten(),
                            entity,
                            location,
                        }
                    });

                    if let Some(component_reg) = type_registry_read_guard
                        .get_or_insert_with(|| type_registry.component.read()) // Lazily gets type registry read lock
                        .get_with_short_name(component_short_name)
                    {
                        // ! FIXME: This function should fail, if the entity doesn't have the component
                        let root_props = component_reg.get_component_properties(
                            &world.archetypes[location.archetype as usize],
                            location.index,
                        );

                        *attr = find_property_ptr(
                            path,
                            root_props,
                            clip.curves[attr_index].value_type(),
                        );

                        // Clear binded bit if not fully binded
                        bind.binded &= *attr != Ptr(null_mut());
                    } else {
                        bind.binded = false;
                        panic!("component `{}` not registered", component_short_name);
                    }
                } else {
                    // Missing entity
                    bind.binded = false;
                }

                // TODO: Check bounds and memory alignment whenever the binds change
                // do this by: sorting the pointers and then checking if the curve value type size
                // doesn't overlap with the next pointer
            }
        }
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
                    panic!("property outside component");
                    //return null_mut();
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
                        if time > clip.duration() {
                            time = (time / clip.duration()).fract() * clip.duration();
                            // Reset all keyframes cached indexes
                            layer
                                .keyframe
                                .iter_mut()
                                .for_each(|x| *x = Default::default())
                        }
                    } else {
                        // Hold
                        time = time.min(clip.duration());
                    }

                    // Advance state time
                    layer.time = time;

                    let w = layer.weight;
                    if w < SMOL {
                        continue;
                    }

                    // TODO: can be done in bached_parallel if is visited warped inside a mutex
                    for (curve_index, (ptr, keyframe)) in bind
                        .pointers
                        .iter()
                        .zip(layer.keyframe.iter_mut())
                        .enumerate()
                    {
                        // Skip invalid properties
                        if *ptr == Ptr(null_mut()) {
                            continue;
                        }

                        let curve = &clip.curves[curve_index];

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
    use crate::curve::Curve;

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

    #[allow(dead_code)]
    struct AnimatorTestBench {
        app: bevy_app::App,
        entities: Vec<Entity>,
        schedule: bevy_ecs::Schedule,
    }

    impl AnimatorTestBench {
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
                clip_a.add_animated_prop("/Node1@Transform.rotation", rot.clone());
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
                clip_b.add_animated_prop("/Node1@Transform.rotation", rot.clone());
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

            let mut entities = vec![];
            entities.push(
                world_builder
                    .spawn(base.clone())
                    .with(Name::from_str("Root"))
                    .with(animator)
                    .current_entity
                    .unwrap(),
            );
            world_builder.with_children(|world_builder| {
                entities.push(
                    world_builder
                        .spawn(base.clone())
                        .with(Name::from_str("Node1"))
                        .current_entity()
                        .unwrap(),
                );

                world_builder.with_children(|world_builder| {
                    entities.push(
                        world_builder
                            .spawn(base.clone())
                            .with(Name::from_str("Node2"))
                            .current_entity()
                            .unwrap(),
                    );

                    world_builder.with_children(|world_builder| {
                        entities.push(
                            world_builder
                                .spawn(base.clone())
                                .with(Name::from_str("Node3"))
                                .current_entity()
                                .unwrap(),
                        );
                    });
                });
            });

            app_builder.set_world(world);

            let mut schedule = bevy_ecs::Schedule::default();
            schedule.add_stage("update");
            schedule.add_stage_after("update", "post_update");
            schedule.add_system_to_stage("update", animator_binding_system);
            schedule.add_system_to_stage("update", animator_update_system);
            schedule.add_system_to_stage("post_update", parent_update_system);
            //schedule.add_system_to_stage("update", transform_propagate_system);

            schedule.initialize(&mut app_builder.app.world, &mut app_builder.app.resources);
            schedule.run(&mut app_builder.app.world, &mut app_builder.app.resources);

            Self {
                app: app_builder.app,
                entities,
                schedule,
            }
        }

        fn run(&mut self) {
            self.schedule
                .run(&mut self.app.world, &mut self.app.resources);
        }

        fn validate_binds<F: Fn(usize, usize, &Ptr)>(&self, validation: F) {
            let animator = self
                .app
                .world
                .get::<Animator>(self.entities[0])
                .expect("missing animator");

            animator
                .bind_clips
                .iter()
                .map(|b| b.as_ref().expect("missing clip bind"))
                .enumerate()
                .for_each(|(i, b)| {
                    b.pointers
                        .iter()
                        .enumerate()
                        .for_each(|(j, attr)| validation(i, j, attr));
                });
        }
    }

    #[test]
    fn entity_deleted() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            assert!(
                *ptr != Ptr(null_mut()),
                "property {} unbinded in curve {}",
                prop_index,
                curve_index,
            )
        });

        // delete "Node1"
        test_bench
            .app
            .world
            .despawn(test_bench.entities[1])
            .unwrap();

        // Tick
        test_bench.run();

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            if prop_index < [2, 2][curve_index] {
                assert!(
                    *ptr != Ptr(null_mut()),
                    "property {} unbinded in curve {}",
                    prop_index,
                    curve_index,
                )
            } else {
                assert!(
                    *ptr == Ptr(null_mut()),
                    "property {} still binded in curve {}",
                    prop_index,
                    curve_index,
                )
            }
        });

        // Re-add "Node1"
        test_bench.entities[1] = test_bench
            .app
            .world
            .build()
            .spawn((
                GlobalTransform::default(),
                Transform::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                Name::from_str("Node1"),
                Parent(test_bench.entities[0]),
            ))
            .current_entity
            .unwrap();

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

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            assert!(
                *ptr != Ptr(null_mut()),
                "property {} wasn't rebinded in curve {}",
                prop_index,
                curve_index,
            )
        });
    }

    #[test]
    fn entity_parent_changed() {
        // Tests for hierarch change deletion and creation of some entity that is been animated
        let mut test_bench = AnimatorTestBench::new();

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            assert!(
                *ptr != Ptr(null_mut()),
                "property {} unbinded in curve {}",
                prop_index,
                curve_index,
            )
        });

        // Change parent of node "Node1"
        let new_parent = test_bench
            .app
            .world
            .build()
            .spawn(())
            .current_entity
            .unwrap();

        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(new_parent);

        // Tick
        test_bench.run();

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            if prop_index < [2, 2][curve_index] {
                assert!(
                    *ptr != Ptr(null_mut()),
                    "property {} unbinded in curve {}",
                    prop_index,
                    curve_index,
                )
            } else {
                assert!(
                    *ptr == Ptr(null_mut()),
                    "property {} still binded in curve {}",
                    prop_index,
                    curve_index,
                )
            }
        });

        // Re-parent "Node1"
        *test_bench
            .app
            .world
            .get_mut::<Parent>(test_bench.entities[1])
            .unwrap() = Parent(test_bench.entities[0]);

        // Tick
        // ! FIXME: May take one frame to bind the missing properties because
        // ! the `parent_update_system` must run in order to update the `Children` component
        test_bench.run();
        test_bench.run();

        test_bench.validate_binds(|curve_index, prop_index, ptr| {
            assert!(
                *ptr != Ptr(null_mut()),
                "property {} wasn't rebinded in curve {}",
                prop_index,
                curve_index,
            )
        });
    }

    #[test]
    fn entity_archetype_change() {
        // Add tests for entity archetype change

        // TODO: Add extra dummy component
        // TODO: Remove dummy component
        // TODO: Remove parent

        unimplemented!()
    }

    #[test]
    fn entity_renamed() {
        unimplemented!()
    }

    #[test]
    fn missing_component_in_entity() {
        // TODO: try to bind a property against a existent entity that doesn't have the target component
        unimplemented!()
    }

    #[test]
    fn entity_with_same_name() {
        // TODO: Add tests for entities with same Name
        unimplemented!()
    }

    #[test]
    fn clip_changed() {
        // TODO: Add tests for mutated events
        unimplemented!()
    }

    #[test]
    fn animated_components_are_marked_as_mutated() {
        // TODO: Add tests for animated components should trigger 'Mutated' queries filters (impl missing)
        unimplemented!()
    }

    #[test]
    fn swapping_handle_animation() {
        // TODO: Add tests for animated handle (impl missing)
        unimplemented!()
    }

    #[test]
    fn animate_asset_properties() {
        // TODO: Add tests for animated asset properties (impl missing)
        unimplemented!()
    }

    #[test]
    #[cfg(feature = "extra-profiling-tests")]
    fn test_bench_update() {
        // ? NOTE: Mimics a basic system update behavior good for pref since criterion will pollute the
        // ? annotations with many expensive instructions
        let mut test_bench = AnimatorTestBench::new();
        test_bench.run();

        for _ in 0..1_000_000 {
            // Time tick
            {
                let mut time = test_bench.app.resources.get_mut::<Time>().unwrap();
                time.delta_seconds += 0.016;
                time.delta_seconds_f64 += 0.016;
            }

            animator_update_system(&mut test_bench.app.world, &mut test_bench.app.resources);
        }
    }

    #[test]
    #[cfg(feature = "extra-profiling-tests")]
    fn test_bench_binding() {
        // ? NOTE: Mimics a basic system update behavior good for pref since criterion will pollute the
        // ? annotations with many expensive instructions
        let mut test_bench = AnimatorTestBench::new();
        test_bench.run();

        for _ in 0..1_000_000 {
            animator_binding_system(&mut test_bench.app.world, &mut test_bench.app.resources);
        }
    }

    // TODO: test update with 5 layers or more ...
}
