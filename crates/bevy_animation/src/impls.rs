use crate as bevy_animation;
use crate::{
    blending::{AnimatorBlending, Blend},
    custom::*,
};
use bevy_animation_derive::*;
use bevy_asset::{Assets, Handle};
use bevy_ecs::prelude::*;
use bevy_math::prelude::*;
use bevy_pbr::prelude::{Light, StandardMaterial};
use bevy_render::{
    camera::{OrthographicProjection, PerspectiveProjection},
    mesh::Mesh,
    prelude::{Color, Texture, Visible},
};
use bevy_transform::prelude::*;

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

                    // SAFETY: Never a different thread will modify or access the same index as this one;
                    // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                    // buckets, this way no cache line will be accessed by the same thread unless the same property
                    // is accessed by two different systems, which is possible but weird and will hit the performance a bit
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
                                //component.translation.blend(&mut blend_group, v, w);
                                // ? NOTE: Blend must be done for each component in order for it to work
                                component.translation.x.blend(&mut blend_group, v.x, w);
                                component.translation.y.blend(&mut blend_group, v.y, w);
                                component.translation.z.blend(&mut blend_group, v.z, w);
                            }
                        }
                    } else {
                        if let Some(curves) = clip
                            .get("Transform.translation.x")
                            .map(|curve_untyped| curve_untyped.downcast_ref::<f32>())
                            .flatten()
                        {
                            for (entity_index, (curve_index, curve)) in curves.iter() {
                                let entity_index = entities_map[entity_index as usize];
                                if let Some(ref mut component) = components[entity_index as usize] {
                                    let (k, v) =
                                        curve.sample_indexed(keyframes[*curve_index], time);
                                    keyframes[*curve_index] = k;
                                    component.translation.x.blend(&mut blend_group, v, w);
                                }
                            }
                        }

                        if let Some(curves) = clip
                            .get("Transform.translation.y")
                            .map(|curve_untyped| curve_untyped.downcast_ref::<f32>())
                            .flatten()
                        {
                            for (entity_index, (curve_index, curve)) in curves.iter() {
                                let entity_index = entities_map[entity_index as usize];
                                if let Some(ref mut component) = components[entity_index as usize] {
                                    let (k, v) =
                                        curve.sample_indexed(keyframes[*curve_index], time);
                                    keyframes[*curve_index] = k;
                                    component.translation.y.blend(&mut blend_group, v, w);
                                }
                            }
                        }

                        if let Some(curves) = clip
                            .get("Transform.translation.z")
                            .map(|curve_untyped| curve_untyped.downcast_ref::<f32>())
                            .flatten()
                        {
                            for (entity_index, (curve_index, curve)) in curves.iter() {
                                let entity_index = entities_map[entity_index as usize];
                                if let Some(ref mut component) = components[entity_index as usize] {
                                    let (k, v) =
                                        curve.sample_indexed(keyframes[*curve_index], time);
                                    keyframes[*curve_index] = k;
                                    component.translation.z.blend(&mut blend_group, v, w);
                                }
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

                    // TODO: Euler rotation support?

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

impl AnimatedAsset for StandardMaterial {
    fn animator_update_system(
        clips: Res<Assets<Clip>>,
        mut animator_blending: Local<AnimatorBlending>,
        animators_query: Query<&Animator>,
        mut assets: ResMut<Assets<Self>>,
        component_query: Query<&mut Handle<Self>>,
    ) {
        let __span = tracing::info_span!("animator_standard_material_update_system");
        let __guard = __span.enter();

        let mut components = vec![];

        for animator in animators_query.iter() {
            let mut blend_group = animator_blending.begin_blending();

            for (_, layer, clip_handle, entities_map) in animator.animate() {
                let w = layer.weight;
                if w < 1.0e-8 {
                    continue;
                }

                components.clear();

                // TODO: Test performance with lazy component fetch
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

                if let Some(clip) = clips.get(clip_handle) {
                    let time = layer.time;

                    // SAFETY: Never a different thread will modify or access the same index as this one;
                    // Plus as a nice and crazy feature each property is grouped by name into their own cache line
                    // buckets, this way no cache line will be accessed by the same thread unless the same property
                    // is accessed by two different systems, which is possible but weird and will hit the performance a bit
                    let keyframes = unsafe { layer.keyframes_unsafe() };

                    // Replace the material handle
                    if let Some(curves) = clip
                        .get("Handle<StandardMaterial>")
                        .map(|curve_untyped| {
                            curve_untyped.downcast_ref::<Handle<StandardMaterial>>()
                        })
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];

                            if let Some(ref mut component) = components[entity_index as usize] {
                                let (k, v) = curve.sample_indexed(keyframes[*curve_index], time);
                                keyframes[*curve_index] = k;
                                component.blend(&mut blend_group, v, w);
                            }
                        }
                    }

                    // Change asset properties

                    if let Some(curves) = clip
                        .get("Handle<StandardMaterial>.albedo")
                        .map(|curve_untyped| curve_untyped.downcast_ref::<Color>())
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];

                            if let Some(ref component) = components[entity_index as usize] {
                                if let Some(asset) = assets.get_mut(&**component) {
                                    let (k, v) =
                                        curve.sample_indexed(keyframes[*curve_index], time);
                                    keyframes[*curve_index] = k;
                                    asset.albedo.blend(&mut blend_group, v, w);
                                }
                            }
                        }
                    }

                    if let Some(curves) = clip
                        .get("Handle<StandardMaterial>.albedo_texture")
                        .map(|curve_untyped| {
                            curve_untyped.downcast_ref::<Option<Handle<Texture>>>()
                        })
                        .flatten()
                    {
                        for (entity_index, (curve_index, curve)) in curves.iter() {
                            let entity_index = entities_map[entity_index as usize];
                            if let Some(ref component) = components[entity_index as usize] {
                                if let Some(asset) = assets.get_mut(&**component) {
                                    let (k, v) =
                                        curve.sample_indexed(keyframes[*curve_index], time);
                                    keyframes[*curve_index] = k;
                                    asset.albedo_texture.blend(&mut blend_group, v, w);
                                }
                            }
                        }
                    }
                }
            }
        }

        std::mem::drop(__guard);
    }
}

// #[derive(Debug, AnimatedComponent)]
// struct Test {
//     #[animated(expand { x: f32, y: f32, z: f32 })]
//     a: Vec3,
//     b: Vec2,
//     #[animated(ignore)]
//     c: Vec3,
// }

// animated_component! {
//     struct Transform {
//         translation: Vec3,
//         rotation: Quat,
//         scale: Vec3,
//     }
// }

// animated_asset! {
//     struct StandardMaterial {
//         albedo: Color,
//         albedo_texture: Option<Handle<Texture>>,
//     }
// }

// animated_asset! {
//     struct Sprite {}
// }

animated_asset! {
    struct Mesh {}
}

animated_component! {
    struct Visible {
        is_visible: bool,
    }
}

animated_component! {
    struct Light {
        color: Color,
        fov: f32,
        //depth: Range<f32>,
    }
}

animated_component! {
    struct PerspectiveProjection {
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    }
}

animated_component! {
    struct OrthographicProjection {
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
        //window_origin: WindowOrigin,
    }
}
