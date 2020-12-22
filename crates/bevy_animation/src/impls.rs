use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_transform::prelude::*;

use crate::blending::{AnimatorBlending, Blend};
use crate::custom::*;

impl AnimatedComponent for Transform {
    fn animator_update_system(
        clips: Res<Assets<Clip>>,
        mut animator_blending: Local<AnimatorBlending>,
        animators_query: Query<&Animator>,
        component_query: Query<&mut Self>,
    ) {
        let __span = tracing::info_span!("animator_transform_update_system");
        let __guard = __span.enter();

        let mut components = vec![];

        for animator in animators_query.iter() {
            let mut blend_group = animator_blending.begin_blending();

            components.clear();

            // ? NOTE: Lazy get each component is worse than just fetching everything at once
            // Pre-fetch all transforms to avoid calling get_mut multiple times
            // SAFETY: each component will be updated one at the time and this function
            // currently has the mutability over the Transform type, so no race conditions
            // are possible
            unsafe {
                for entity in animator.entities() {
                    components.push(
                        entity
                            .map(|entity| component_query.get_unsafe(entity).ok())
                            .flatten(),
                    );
                }
            }

            for (_, layer, clip_handle, entities_map) in animator.animate() {
                let w = layer.weight;
                if w < 1.0e-8 {
                    continue;
                }

                if let Some(clip) = clips.get(clip_handle) {
                    let time = layer.time;

                    // SAFETY: Never a different thread will modify or access the same index as this one
                    // ! FIXME: Multiple threads will modifying the same cache line, not sure if this will cause any issue
                    let keyframes = unsafe { layer.keyframes_unsafe() };

                    if let Some(curves) = clip
                        .get("Transform.translation")
                        .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];
                            if let Some(ref mut component) = components[entity_index as usize] {
                                let (k, v) = curve.sample_indexed(keyframes[*curve_index], time);
                                keyframes[*curve_index] = k;
                                component.translation.blend(&mut blend_group, v, w);
                            }
                        }
                    }

                    if let Some(curves) = clip
                        .get("Transform.rotation")
                        .map(|curve_untyped| curve_untyped.downcast_ref::<Quat>())
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];
                            if let Some(ref mut component) = components[entity_index as usize] {
                                let (k, v) = curve.sample_indexed(keyframes[*curve_index], time);
                                keyframes[*curve_index] = k;
                                component.rotation.blend(&mut blend_group, v, w);
                            }
                        }
                    }

                    if let Some(curves) = clip
                        .get("Transform.scale")
                        .map(|curve_untyped| curve_untyped.downcast_ref::<Vec3>())
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];
                            if let Some(ref mut component) = components[entity_index as usize] {
                                let (k, v) = curve.sample_indexed(keyframes[*curve_index], time);
                                keyframes[*curve_index] = k;
                                component.scale.blend(&mut blend_group, v, w);
                            }
                        }
                    }
                }
            }
        }

        std::mem::drop(__guard);
    }
}
