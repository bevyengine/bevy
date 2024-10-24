use bevy_animation::{
    gltf_curves::{
        CubicKeyframeCurve, CubicRotationCurve, SteppedKeyframeCurve, WideCubicKeyframeCurve,
        WideLinearKeyframeCurve, WideSteppedKeyframeCurve,
    },
    prelude::{RotationCurve, ScaleCurve, TranslationCurve, WeightsCurve},
    AnimationClip, AnimationTargetId, VariableCurve,
};
use bevy_core::Name;
use bevy_math::{
    curve::{constant_curve, Interval, UnevenSampleAutoCurve},
    Quat, Vec3, Vec4,
};
use bevy_utils::{tracing::warn, HashMap, HashSet};
use gltf::animation::util::ReadOutputs;

use crate::GltfError;

pub trait AnimationExt {
    #[allow(clippy::result_large_err)]
    fn load_animation(
        &self,
        buffer_data: &[Vec<u8>],
        animation_paths: &HashMap<usize, (usize, Vec<Name>)>,
        animation_roots: &mut HashSet<usize>,
    ) -> Result<AnimationClip, GltfError>;
}

impl AnimationExt for gltf::Animation<'_> {
    fn load_animation(
        &self,
        buffer_data: &[Vec<u8>],
        animation_paths: &HashMap<usize, (usize, Vec<Name>)>,
        animation_roots: &mut HashSet<usize>,
    ) -> Result<AnimationClip, GltfError> {
        let mut animation_clip = AnimationClip::default();

        for channel in self.channels() {
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
                return Err(GltfError::MissingAnimationSampler(self.index()));
            };

            if keyframe_timestamps.is_empty() {
                warn!("Tried to load animation with no keyframe timestamps");
                continue;
            }

            let maybe_curve: Option<VariableCurve> = if let Some(outputs) = reader.read_outputs() {
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
                                gltf::animation::Interpolation::Step => SteppedKeyframeCurve::new(
                                    keyframe_timestamps.into_iter().zip(translations),
                                )
                                .ok()
                                .map(TranslationCurve)
                                .map(VariableCurve::new),
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
                        let rotations: Vec<Quat> = rots.into_f32().map(Quat::from_array).collect();
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
                                gltf::animation::Interpolation::Step => SteppedKeyframeCurve::new(
                                    keyframe_timestamps.into_iter().zip(rotations),
                                )
                                .ok()
                                .map(RotationCurve)
                                .map(VariableCurve::new),
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
                                gltf::animation::Interpolation::Step => SteppedKeyframeCurve::new(
                                    keyframe_timestamps.into_iter().zip(scales),
                                )
                                .ok()
                                .map(ScaleCurve)
                                .map(VariableCurve::new),
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
                return Err(GltfError::MissingAnimationSampler(self.index()));
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

        Ok(animation_clip)
    }
}
