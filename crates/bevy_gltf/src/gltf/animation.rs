use bevy_animation::{animation_curves::*, gltf_curves::*, VariableCurve};
use bevy_animation::{AnimationClip, AnimationTargetId};
use bevy_asset::{Handle, LoadContext};
use bevy_core::Name;
use bevy_math::curve::{constant_curve, Interval, UnevenSampleAutoCurve};
use bevy_math::Vec3;
use bevy_math::{Quat, Vec4};
use bevy_utils::{tracing::warn, HashMap, HashSet};
use gltf::animation::util::ReadOutputs;

use crate::{ext::NodeExt, GltfAssetLabel, GltfError};

#[derive(Debug)]
/// An animation in a [`glTF`](gltf::Gltf)
pub struct GltfAnimation;

impl GltfAnimation {
    #[allow(clippy::result_large_err)]
    /// Loads all animation in a [`glTF`](gltf::Gltf).
    ///
    /// Returns a list of [`GltfAnimation`]s and a set of the animation roots.
    pub(crate) fn load_animations(
        load_context: &mut LoadContext,
        gltf: &gltf::Gltf,
        buffer_data: &[Vec<u8>],
    ) -> Result<
        (
            Vec<Handle<AnimationClip>>,
            HashMap<Box<str>, Handle<AnimationClip>>,
            HashSet<usize>,
        ),
        GltfError,
    > {
        let animation_paths = Self::load_animation_paths(gltf);

        let mut animations = vec![];
        let mut named_animations = HashMap::new();
        let mut animation_roots = HashSet::new();

        for animation in gltf.animations() {
            let mut animation_clip = AnimationClip::default();
            for channel in animation.channels() {
                let node = channel.target().node();
                let interpolation = channel.sampler().interpolation();
                let reader = channel.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let keyframe_timestamps: Vec<f32> = if let Some(inputs) = reader.read_inputs() {
                    match inputs {
                        gltf::accessor::Iter::Standard(times) => times.collect(),
                        gltf::accessor::Iter::Sparse(_) => {
                            warn!("Sparse accessor not supported for animation sampler input");
                            continue;
                        }
                    }
                } else {
                    warn!("Animations without a sampler input are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                if keyframe_timestamps.is_empty() {
                    warn!("Tried to load animation with no keyframe timestamps");
                    continue;
                }

                let maybe_curve: Option<VariableCurve> = if let Some(outputs) =
                    reader.read_outputs()
                {
                    match outputs {
                        ReadOutputs::Translations(tr) => {
                            let translations: Vec<Vec3> = tr.map(Vec3::from).collect();
                            if keyframe_timestamps.len() == 1 {
                                #[allow(clippy::unnecessary_map_on_constructor)]
                                Some(constant_curve(Interval::EVERYWHERE, translations[0]))
                                    .map(TranslationCurve)
                                    .map(VariableCurve::new)
                            } else {
                                match interpolation {
                                    gltf::animation::Interpolation::Linear => {
                                        UnevenSampleAutoCurve::new(
                                            keyframe_timestamps.into_iter().zip(translations),
                                        )
                                        .ok()
                                        .map(TranslationCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::Step => {
                                        SteppedKeyframeCurve::new(
                                            keyframe_timestamps.into_iter().zip(translations),
                                        )
                                        .ok()
                                        .map(TranslationCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::CubicSpline => {
                                        CubicKeyframeCurve::new(keyframe_timestamps, translations)
                                            .ok()
                                            .map(TranslationCurve)
                                            .map(VariableCurve::new)
                                    }
                                }
                            }
                        }
                        ReadOutputs::Rotations(rots) => {
                            let rotations: Vec<Quat> =
                                rots.into_f32().map(Quat::from_array).collect();
                            if keyframe_timestamps.len() == 1 {
                                #[allow(clippy::unnecessary_map_on_constructor)]
                                Some(constant_curve(Interval::EVERYWHERE, rotations[0]))
                                    .map(RotationCurve)
                                    .map(VariableCurve::new)
                            } else {
                                match interpolation {
                                    gltf::animation::Interpolation::Linear => {
                                        UnevenSampleAutoCurve::new(
                                            keyframe_timestamps.into_iter().zip(rotations),
                                        )
                                        .ok()
                                        .map(RotationCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::Step => {
                                        SteppedKeyframeCurve::new(
                                            keyframe_timestamps.into_iter().zip(rotations),
                                        )
                                        .ok()
                                        .map(RotationCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::CubicSpline => {
                                        CubicRotationCurve::new(
                                            keyframe_timestamps,
                                            rotations.into_iter().map(Vec4::from),
                                        )
                                        .ok()
                                        .map(RotationCurve)
                                        .map(VariableCurve::new)
                                    }
                                }
                            }
                        }
                        ReadOutputs::Scales(scale) => {
                            let scales: Vec<Vec3> = scale.map(Vec3::from).collect();
                            if keyframe_timestamps.len() == 1 {
                                #[allow(clippy::unnecessary_map_on_constructor)]
                                Some(constant_curve(Interval::EVERYWHERE, scales[0]))
                                    .map(ScaleCurve)
                                    .map(VariableCurve::new)
                            } else {
                                match interpolation {
                                    gltf::animation::Interpolation::Linear => {
                                        UnevenSampleAutoCurve::new(
                                            keyframe_timestamps.into_iter().zip(scales),
                                        )
                                        .ok()
                                        .map(ScaleCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::Step => {
                                        SteppedKeyframeCurve::new(
                                            keyframe_timestamps.into_iter().zip(scales),
                                        )
                                        .ok()
                                        .map(ScaleCurve)
                                        .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::CubicSpline => {
                                        CubicKeyframeCurve::new(keyframe_timestamps, scales)
                                            .ok()
                                            .map(ScaleCurve)
                                            .map(VariableCurve::new)
                                    }
                                }
                            }
                        }
                        ReadOutputs::MorphTargetWeights(weights) => {
                            let weights: Vec<f32> = weights.into_f32().collect();
                            if keyframe_timestamps.len() == 1 {
                                #[allow(clippy::unnecessary_map_on_constructor)]
                                Some(constant_curve(Interval::EVERYWHERE, weights))
                                    .map(WeightsCurve)
                                    .map(VariableCurve::new)
                            } else {
                                match interpolation {
                                    gltf::animation::Interpolation::Linear => {
                                        WideLinearKeyframeCurve::new(keyframe_timestamps, weights)
                                            .ok()
                                            .map(WeightsCurve)
                                            .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::Step => {
                                        WideSteppedKeyframeCurve::new(keyframe_timestamps, weights)
                                            .ok()
                                            .map(WeightsCurve)
                                            .map(VariableCurve::new)
                                    }
                                    gltf::animation::Interpolation::CubicSpline => {
                                        WideCubicKeyframeCurve::new(keyframe_timestamps, weights)
                                            .ok()
                                            .map(WeightsCurve)
                                            .map(VariableCurve::new)
                                    }
                                }
                            }
                        }
                    }
                } else {
                    warn!("Animations without a sampler output are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                let Some(curve) = maybe_curve else {
                    warn!(
                        "Invalid keyframe data for node {}; curve could not be constructed",
                        node.index()
                    );
                    continue;
                };

                if let Some((root_index, path)) = animation_paths.get(&node.index()) {
                    animation_roots.insert(*root_index);
                    animation_clip.add_variable_curve_to_target(
                        AnimationTargetId::from_names(path.iter()),
                        curve,
                    );
                } else {
                    warn!(
                        "Animation ignored for node {}: part of its hierarchy is missing a name",
                        node.index()
                    );
                }
            }
            let handle = load_context.add_labeled_asset(
                GltfAssetLabel::Animation(animation.index()).to_string(),
                animation_clip,
            );
            if let Some(name) = animation.name() {
                named_animations.insert(name.into(), handle.clone());
            }
            animations.push(handle);
        }

        Ok((animations, named_animations, animation_roots))
    }

    fn load_animation_paths(gltf: &gltf::Gltf) -> HashMap<usize, (usize, Vec<Name>)> {
        let mut paths = HashMap::new();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                let root_index = node.index();
                Self::paths_recur(node, &[], &mut paths, root_index, &mut HashSet::new());
            }
        }
        paths
    }

    fn paths_recur(
        node: gltf::Node,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    ) {
        let mut path = current_path.to_owned();
        path.push(node.to_name());
        visited.insert(node.index());
        for child in node.children() {
            if !visited.contains(&child.index()) {
                Self::paths_recur(child, &path, paths, root_index, visited);
            }
        }
        paths.insert(node.index(), (root_index, path));
    }
}
