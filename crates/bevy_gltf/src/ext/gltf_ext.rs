use bevy_animation::{
    gltf_curves::{
        CubicKeyframeCurve, CubicRotationCurve, SteppedKeyframeCurve, WideCubicKeyframeCurve,
        WideLinearKeyframeCurve, WideSteppedKeyframeCurve,
    },
    prelude::{RotationCurve, ScaleCurve, TranslationCurve, WeightsCurve},
    AnimationClip, AnimationTargetId, VariableCurve,
};
use bevy_asset::{Handle, LoadContext};
use bevy_core::Name;
use bevy_math::{
    curve::{constant_curve, Interval, UnevenSampleAutoCurve},
    Mat4, Quat, Vec3, Vec4,
};
use bevy_pbr::StandardMaterial;
use bevy_render::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy_utils::{tracing::warn, HashMap, HashSet};
use gltf::animation::util::ReadOutputs;

use crate::{data_uri::DataUri, GltfAssetLabel, GltfError, GltfLoaderSettings};

use super::{MaterialExt, NodeExt, SkinExt};

const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

/// [`glTF`](gltf::Gltf) extension
pub trait GltfExt {
    /// Loads the raw glTF buffer data for a specific glTF file.
    async fn load_buffers(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Vec<u8>>, GltfError>;

    #[cfg(feature = "bevy_animation")]
    #[allow(clippy::result_large_err)]
    /// Loads all animation in a [`glTF`](gltf::Gltf).
    ///
    /// Returns a list of [`Handles`](Handle) to [`AnimationClips`](AnimationClip) and a set of the animation roots.
    fn load_animations(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Result<
        (
            Vec<Handle<AnimationClip>>,
            HashMap<Box<str>, Handle<AnimationClip>>,
            HashSet<usize>,
        ),
        GltfError,
    >;

    #[allow(clippy::result_large_err)]
    /// Loads all materials found on [`glTF`](gltf::Gltf).
    fn load_materials(
        &self,
        load_context: &mut LoadContext<'_>,
        settings: &GltfLoaderSettings,
    ) -> Result<
        (
            Vec<Handle<StandardMaterial>>,
            HashMap<Box<str>, Handle<StandardMaterial>>,
        ),
        GltfError,
    >;

    /// Load [`SkinnedMeshInverseBindposes`] for all [`Skin`](gltf::Skin)
    /// in [`glTF`](gltf::Gltf).
    fn inverse_bind_poses(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Vec<Handle<SkinnedMeshInverseBindposes>>;

    /// Get index of [`Meshs`](gltf::Mesh) that are on skinned or non-skinned [`Nodes`](gltf::Node)
    fn load_meshes_on_nodes(&self) -> (HashSet<usize>, HashSet<usize>);

    /// Get index of [`Textures`](gltf::texture::Texture) that are used by
    /// a [`Material`](gltf::Material)
    fn textures_used_by_materials(&self) -> HashSet<usize>;

    fn load_animation_paths(&self) -> HashMap<usize, (usize, Vec<Name>)>;
}

impl GltfExt for gltf::Gltf {
    async fn load_buffers(
        &self,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Vec<Vec<u8>>, GltfError> {
        let mut buffer_data = Vec::new();
        for buffer in self.buffers() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) => {
                    let uri = percent_encoding::percent_decode_str(uri)
                        .decode_utf8()
                        .unwrap();
                    let uri = uri.as_ref();
                    let buffer_bytes = match DataUri::parse(uri) {
                        Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                            data_uri.decode()?
                        }
                        Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                        Err(()) => {
                            // TODO: Remove this and add dep
                            let buffer_path = load_context.path().parent().unwrap().join(uri);
                            load_context.read_asset_bytes(buffer_path).await?
                        }
                    };
                    buffer_data.push(buffer_bytes);
                }
                gltf::buffer::Source::Bin => {
                    if let Some(blob) = self.blob.as_deref() {
                        buffer_data.push(blob.into());
                    } else {
                        return Err(GltfError::MissingBlob);
                    }
                }
            }
        }

        Ok(buffer_data)
    }

    #[cfg(feature = "bevy_animation")]
    #[allow(clippy::result_large_err)]
    fn load_animations(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Result<
        (
            Vec<Handle<AnimationClip>>,
            HashMap<Box<str>, Handle<AnimationClip>>,
            HashSet<usize>,
        ),
        GltfError,
    > {
        let animation_paths = self.load_animation_paths();

        let mut animations = vec![];
        let mut named_animations = HashMap::new();
        let mut animation_roots = HashSet::new();

        for animation in self.animations() {
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

    #[allow(clippy::result_large_err)]
    fn load_materials(
        &self,
        load_context: &mut LoadContext<'_>,
        settings: &GltfLoaderSettings,
    ) -> Result<
        (
            Vec<Handle<StandardMaterial>>,
            HashMap<Box<str>, Handle<StandardMaterial>>,
        ),
        GltfError,
    > {
        let mut materials = vec![];
        let mut named_materials = HashMap::new();

        // Only include materials in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_materials flag
        if !settings.load_materials.is_empty() {
            // NOTE: materials must be loaded after textures because image load() calls will
            // happen before load_with_settings, preventing is_srgb from being set properly
            for material in self.materials() {
                let handle = material.load_material(load_context, &self.document, false);
                if let Some(name) = material.name() {
                    named_materials.insert(name.into(), handle.clone());
                }
                materials.push(handle);
            }
        }

        Ok((materials, named_materials))
    }

    fn inverse_bind_poses(
        &self,
        load_context: &mut LoadContext,
        buffer_data: &[Vec<u8>],
    ) -> Vec<Handle<SkinnedMeshInverseBindposes>> {
        self.skins()
            .map(|gltf_skin| {
                let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let local_to_bone_bind_matrices: Vec<Mat4> = reader
                    .read_inverse_bind_matrices()
                    .unwrap()
                    .map(|mat| Mat4::from_cols_array_2d(&mat))
                    .collect();

                load_context.add_labeled_asset(
                    gltf_skin.inverse_bind_matrices_label().to_string(),
                    SkinnedMeshInverseBindposes::from(local_to_bone_bind_matrices),
                )
            })
            .collect()
    }

    fn load_meshes_on_nodes(&self) -> (HashSet<usize>, HashSet<usize>) {
        let mut meshes_on_skinned_nodes = HashSet::default();
        let mut meshes_on_non_skinned_nodes = HashSet::default();

        for gltf_node in self.nodes() {
            if gltf_node.is_skinned() {
                if let Some(mesh_index) = gltf_node.mesh_index() {
                    meshes_on_skinned_nodes.insert(mesh_index);
                }
            } else if let Some(mesh_index) = gltf_node.mesh_index() {
                meshes_on_non_skinned_nodes.insert(mesh_index);
            }
        }

        (meshes_on_skinned_nodes, meshes_on_non_skinned_nodes)
    }

    fn textures_used_by_materials(&self) -> HashSet<usize> {
        let mut used_textures = HashSet::default();

        for material in self.materials() {
            if let Some(normal_texture_index) = material.normal_texture_index() {
                used_textures.insert(normal_texture_index);
            }
            if let Some(occlusion_texture_index) = material.occlusion_texture_index() {
                used_textures.insert(occlusion_texture_index);
            }
            if let Some(metallic_roughness_texture_index) =
                material.metallic_roughness_texture_index()
            {
                used_textures.insert(metallic_roughness_texture_index);
            }

            #[cfg(feature = "pbr_anisotropy_texture")]
            if let Some(texture_index) =
                material.extension_texture_index("KHR_materials_anisotropy", "anisotropyTexture")
            {
                used_textures.insert(texture_index);
            }

            // None of the clearcoat maps should be loaded as sRGB.
            #[cfg(feature = "pbr_multi_layer_material_textures")]
            for texture_field_name in [
                "clearcoatTexture",
                "clearcoatRoughnessTexture",
                "clearcoatNormalTexture",
            ] {
                if let Some(texture_index) =
                    material.extension_texture_index("KHR_materials_clearcoat", texture_field_name)
                {
                    used_textures.insert(texture_index);
                }
            }
        }

        used_textures
    }

    fn load_animation_paths(&self) -> HashMap<usize, (usize, Vec<Name>)> {
        let mut paths = HashMap::new();
        for scene in self.scenes() {
            for node in scene.nodes() {
                let root_index = node.index();
                node.paths_recur(&[], &mut paths, root_index, &mut HashSet::new());
            }
        }
        paths
    }
}
