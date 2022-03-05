use anyhow::Result;
use bevy_asset::{
    AssetIoError, AssetLoader, AssetPath, BoxedFuture, Handle, LoadContext, LoadedAsset,
};
use bevy_core::Name;
use bevy_ecs::{prelude::FromWorld, world::World};
use bevy_hierarchy::{BuildWorldChildren, WorldChildBuilder};
use bevy_log::warn;
use bevy_math::{Mat4, Quat, Vec3};
use bevy_pbr::{
    AlphaMode, DirectionalLight, DirectionalLightBundle, PbrBundle, PointLight, PointLightBundle,
    StandardMaterial,
};
use bevy_render::{
    camera::{
        Camera, Camera2d, Camera3d, CameraProjection, OrthographicProjection, PerspectiveProjection,
    },
    color::Color,
    mesh::{Indices, Mesh, VertexAttributeValues},
    primitives::{Aabb, Frustum},
    render_resource::{AddressMode, FilterMode, PrimitiveTopology, SamplerDescriptor},
    renderer::RenderDevice,
    texture::{CompressedImageFormats, Image, ImageType, TextureError},
    view::VisibleEntities,
};
use bevy_scene::Scene;
use bevy_transform::{components::Transform, TransformBundle};

use bevy_utils::{HashMap, HashSet};
use gltf::{
    mesh::Mode,
    texture::{MagFilter, MinFilter, WrappingMode},
    Material, Primitive,
};
use std::{collections::VecDeque, path::Path};
use thiserror::Error;

use crate::{
    Gltf, GltfAnimatedNode, GltfAnimation, GltfAnimationInterpolation, GltfNode, GltfNodeAnimation,
    GltfNodeAnimationKeyframes,
};

/// An error that occurs when loading a glTF file.
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive { mode: Mode },
    #[error("invalid glTF file: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("binary blob is missing")]
    MissingBlob,
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    #[error("You may need to add the feature for the file format: {0}")]
    ImageError(#[from] TextureError),
    #[error("failed to load an asset path: {0}")]
    AssetIoError(#[from] AssetIoError),
    #[error("Missing sampler for animation {0}")]
    MissingAnimationSampler(usize),
}

/// Loads glTF files with all of their data as their corresponding bevy representations.
pub struct GltfLoader {
    supported_compressed_formats: CompressedImageFormats,
}

impl AssetLoader for GltfLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move {
            Ok(load_gltf(bytes, load_context, self.supported_compressed_formats).await?)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["gltf", "glb"]
    }
}

impl FromWorld for GltfLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            supported_compressed_formats: CompressedImageFormats::from_features(
                world.resource::<RenderDevice>().features(),
            ),
        }
    }
}

/// Loads an entire glTF file.
async fn load_gltf<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
    supported_compressed_formats: CompressedImageFormats,
) -> Result<(), GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let buffer_data = load_buffers(&gltf, load_context, load_context.path()).await?;

    let mut materials = vec![];
    let mut named_materials = HashMap::default();
    let mut linear_textures = HashSet::default();
    for material in gltf.materials() {
        let handle = load_material(&material, load_context);
        if let Some(name) = material.name() {
            named_materials.insert(name.to_string(), handle.clone());
        }
        materials.push(handle);
        if let Some(texture) = material.normal_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material.occlusion_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            linear_textures.insert(texture.texture().index());
        }
    }

    let mut animations = vec![];
    let mut named_animations = HashMap::default();
    let mut animated_nodes = HashSet::default();
    for animation in gltf.animations() {
        let mut gltf_animation = GltfAnimation::default();
        for channel in animation.channels() {
            let interpolation = match channel.sampler().interpolation() {
                gltf::animation::Interpolation::Linear => GltfAnimationInterpolation::Linear,
                gltf::animation::Interpolation::Step => GltfAnimationInterpolation::Step,
                gltf::animation::Interpolation::CubicSpline => {
                    GltfAnimationInterpolation::CubicSpline
                }
            };
            let node = channel.target().node();
            let reader = channel.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let keyframe_timestamps: Vec<f32> = if let Some(inputs) = reader.read_inputs() {
                match inputs {
                    gltf::accessor::Iter::Standard(times) => times.collect(),
                    gltf::accessor::Iter::Sparse(_) => {
                        warn!("sparse accessor not supported for animation sampler input");
                        continue;
                    }
                }
            } else {
                warn!("animations without a sampler input are not supported");
                return Err(GltfError::MissingAnimationSampler(animation.index()));
            };

            let keyframes = if let Some(outputs) = reader.read_outputs() {
                match outputs {
                    gltf::animation::util::ReadOutputs::Translations(tr) => {
                        GltfNodeAnimationKeyframes::Translation(tr.map(Vec3::from).collect())
                    }
                    gltf::animation::util::ReadOutputs::Rotations(rots) => {
                        GltfNodeAnimationKeyframes::Rotation(
                            rots.into_f32().map(Quat::from_array).collect(),
                        )
                    }
                    gltf::animation::util::ReadOutputs::Scales(scale) => {
                        GltfNodeAnimationKeyframes::Scale(scale.map(Vec3::from).collect())
                    }
                    gltf::animation::util::ReadOutputs::MorphTargetWeights(_) => {
                        warn!("Morph animation property not yet supported");
                        continue;
                    }
                }
            } else {
                warn!("animations without a sampler output are not supported");
                return Err(GltfError::MissingAnimationSampler(animation.index()));
            };

            gltf_animation
                .node_animations
                .entry(node.index())
                .or_default()
                .push(GltfNodeAnimation {
                    keyframe_timestamps,
                    keyframes,
                    interpolation,
                });
            animated_nodes.insert(node.index());
        }
        let handle = load_context.set_labeled_asset(
            &format!("Animation{}", animation.index()),
            LoadedAsset::new(gltf_animation),
        );
        if let Some(name) = animation.name() {
            named_animations.insert(name.to_string(), handle.clone());
        }
        animations.push(handle);
    }

    let mut meshes = vec![];
    let mut named_meshes = HashMap::default();
    for mesh in gltf.meshes() {
        let mut primitives = vec![];
        for primitive in mesh.primitives() {
            let primitive_label = primitive_label(&mesh, &primitive);
            let reader = primitive.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology);

            if let Some(vertex_attribute) = reader
                .read_positions()
                .map(|v| VertexAttributeValues::Float32x3(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_normals()
                .map(|v| VertexAttributeValues::Float32x3(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_tangents()
                .map(|v| VertexAttributeValues::Float32x4(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
            }

            if let Some(vertex_attribute) = reader
                .read_tex_coords(0)
                .map(|v| VertexAttributeValues::Float32x2(v.into_f32().collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, vertex_attribute);
            } else {
                let len = mesh.count_vertices();
                let uvs = vec![[0.0, 0.0]; len];
                bevy_log::debug!("missing `TEXCOORD_0` vertex attribute, loading zeroed out UVs");
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
            }

            // if let Some(vertex_attribute) = reader
            //     .read_colors(0)
            //     .map(|v| VertexAttributeValues::Float32x4(v.into_rgba_f32().collect()))
            // {
            //     mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_attribute);
            // }

            if let Some(indices) = reader.read_indices() {
                mesh.set_indices(Some(Indices::U32(indices.into_u32().collect())));
            };

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none() {
                let vertex_count_before = mesh.count_vertices();
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let vertex_count_after = mesh.count_vertices();

                if vertex_count_before != vertex_count_after {
                    bevy_log::debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
                } else {
                    bevy_log::debug!(
                        "Missing vertex normals in indexed geometry, computing them as flat."
                    );
                }
            }

            let mesh = load_context.set_labeled_asset(&primitive_label, LoadedAsset::new(mesh));
            primitives.push(super::GltfPrimitive {
                mesh,
                material: primitive
                    .material()
                    .index()
                    .and_then(|i| materials.get(i).cloned()),
            });
        }
        let handle = load_context.set_labeled_asset(
            &mesh_label(&mesh),
            LoadedAsset::new(super::GltfMesh { primitives }),
        );
        if let Some(name) = mesh.name() {
            named_meshes.insert(name.to_string(), handle.clone());
        }
        meshes.push(handle);
    }

    let mut nodes_intermediate = vec![];
    let mut named_nodes_intermediate = HashMap::default();
    for node in gltf.nodes() {
        let node_label = node_label(&node);
        nodes_intermediate.push((
            node_label,
            GltfNode {
                children: vec![],
                mesh: node
                    .mesh()
                    .map(|mesh| mesh.index())
                    .and_then(|i| meshes.get(i).cloned()),
                transform: match node.transform() {
                    gltf::scene::Transform::Matrix { matrix } => {
                        Transform::from_matrix(bevy_math::Mat4::from_cols_array_2d(&matrix))
                    }
                    gltf::scene::Transform::Decomposed {
                        translation,
                        rotation,
                        scale,
                    } => Transform {
                        translation: bevy_math::Vec3::from(translation),
                        rotation: bevy_math::Quat::from_vec4(rotation.into()),
                        scale: bevy_math::Vec3::from(scale),
                    },
                },
            },
            node.children()
                .map(|child| child.index())
                .collect::<Vec<_>>(),
        ));
        if let Some(name) = node.name() {
            named_nodes_intermediate.insert(name, node.index());
        }
    }
    let nodes = resolve_node_hierarchy(nodes_intermediate, load_context.path())
        .into_iter()
        .map(|(label, node)| load_context.set_labeled_asset(&label, LoadedAsset::new(node)))
        .collect::<Vec<bevy_asset::Handle<GltfNode>>>();
    let named_nodes = named_nodes_intermediate
        .into_iter()
        .filter_map(|(name, index)| {
            nodes
                .get(index)
                .map(|handle| (name.to_string(), handle.clone()))
        })
        .collect();

    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    // See https://github.com/bevyengine/bevy/issues/1924 for more details
    // The taskpool use is also avoided when there is only one texture for performance reasons and
    // to avoid https://github.com/bevyengine/bevy/pull/2725
    if gltf.textures().len() == 1 || cfg!(target_arch = "wasm32") {
        for gltf_texture in gltf.textures() {
            let (texture, label) = load_texture(
                gltf_texture,
                &buffer_data,
                &linear_textures,
                load_context,
                supported_compressed_formats,
            )
            .await?;
            load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        load_context
            .task_pool()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let linear_textures = &linear_textures;
                    let load_context: &LoadContext = load_context;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        load_texture(
                            gltf_texture,
                            buffer_data,
                            linear_textures,
                            load_context,
                            supported_compressed_formats,
                        )
                        .await
                    });
                });
            })
            .into_iter()
            .filter_map(|res| {
                if let Err(err) = res.as_ref() {
                    warn!("Error loading glTF texture: {}", err);
                }
                res.ok()
            })
            .for_each(|(texture, label)| {
                load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
            });
    }

    let mut scenes = vec![];
    let mut named_scenes = HashMap::default();
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        world
            .spawn()
            .insert_bundle(TransformBundle::identity())
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result =
                        load_node(&node, parent, load_context, &buffer_data, &animated_nodes);
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            });
        if let Some(Err(err)) = err {
            return Err(err);
        }
        let scene_handle = load_context
            .set_labeled_asset(&scene_label(&scene), LoadedAsset::new(Scene::new(world)));

        if let Some(name) = scene.name() {
            named_scenes.insert(name.to_string(), scene_handle.clone());
        }
        scenes.push(scene_handle);
    }

    load_context.set_default_asset(LoadedAsset::new(Gltf {
        default_scene: gltf
            .default_scene()
            .and_then(|scene| scenes.get(scene.index()))
            .cloned(),
        scenes,
        named_scenes,
        meshes,
        named_meshes,
        materials,
        named_materials,
        nodes,
        named_nodes,
        animations,
        named_animations,
    }));

    Ok(())
}

/// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
async fn load_texture<'a>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
    load_context: &LoadContext<'a>,
    supported_compressed_formats: CompressedImageFormats,
) -> Result<(Image, String), GltfError> {
    let is_srgb = !linear_textures.contains(&gltf_texture.index());
    let mut texture = match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset() as usize;
            let end = (view.offset() + view.length()) as usize;
            let buffer = &buffer_data[view.buffer().index()][start..end];
            Image::from_buffer(
                buffer,
                ImageType::MimeType(mime_type),
                supported_compressed_formats,
                is_srgb,
            )?
        }
        gltf::image::Source::Uri { uri, mime_type } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            let (bytes, image_type) = if let Ok(data_uri) = DataUri::parse(uri) {
                (data_uri.decode()?, ImageType::MimeType(data_uri.mime_type))
            } else {
                let parent = load_context.path().parent().unwrap();
                let image_path = parent.join(uri);
                let bytes = load_context.read_asset_bytes(image_path.clone()).await?;

                let extension = Path::new(uri).extension().unwrap().to_str().unwrap();
                let image_type = ImageType::Extension(extension);

                (bytes, image_type)
            };

            Image::from_buffer(
                &bytes,
                mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                supported_compressed_formats,
                is_srgb,
            )?
        }
    };
    texture.sampler_descriptor = texture_sampler(&gltf_texture);

    Ok((texture, texture_label(&gltf_texture)))
}

/// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
fn load_material(material: &Material, load_context: &mut LoadContext) -> Handle<StandardMaterial> {
    let material_label = material_label(material);

    let pbr = material.pbr_metallic_roughness();

    let color = pbr.base_color_factor();
    let base_color_texture = if let Some(info) = pbr.base_color_texture() {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let normal_map_texture: Option<Handle<Image>> =
        if let Some(normal_texture) = material.normal_texture() {
            // TODO: handle normal_texture.scale
            // TODO: handle normal_texture.tex_coord() (the *set* index for the right texcoords)
            let label = texture_label(&normal_texture.texture());
            let path = AssetPath::new_ref(load_context.path(), Some(&label));
            Some(load_context.get_handle(path))
        } else {
            None
        };

    let metallic_roughness_texture = if let Some(info) = pbr.metallic_roughness_texture() {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let occlusion_texture = if let Some(occlusion_texture) = material.occlusion_texture() {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&occlusion_texture.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    let emissive = material.emissive_factor();
    let emissive_texture = if let Some(info) = material.emissive_texture() {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        Some(load_context.get_handle(path))
    } else {
        None
    };

    load_context.set_labeled_asset(
        &material_label,
        LoadedAsset::new(StandardMaterial {
            base_color: Color::rgba(color[0], color[1], color[2], color[3]),
            base_color_texture,
            perceptual_roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_texture,
            normal_map_texture,
            double_sided: material.double_sided(),
            occlusion_texture,
            emissive: Color::rgba(emissive[0], emissive[1], emissive[2], 1.0),
            emissive_texture,
            unlit: material.unlit(),
            alpha_mode: alpha_mode(material),
            ..Default::default()
        }),
    )
}

/// Loads a glTF node.
fn load_node(
    gltf_node: &gltf::Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    buffer_data: &[Vec<u8>],
    animated_nodes: &HashSet<usize>,
) -> Result<(), GltfError> {
    let transform = gltf_node.transform();
    let mut gltf_error = None;
    let mut node = world_builder.spawn_bundle(TransformBundle::from(Transform::from_matrix(
        Mat4::from_cols_array_2d(&transform.matrix()),
    )));

    if animated_nodes.contains(&gltf_node.index()) {
        node.insert(GltfAnimatedNode {
            index: gltf_node.index(),
        });
    }

    if let Some(name) = gltf_node.name() {
        node.insert(Name::new(name.to_string()));
    }

    // create camera node
    if let Some(camera) = gltf_node.camera() {
        node.insert_bundle((
            VisibleEntities {
                ..Default::default()
            },
            Frustum::default(),
        ));

        match camera.projection() {
            gltf::camera::Projection::Orthographic(orthographic) => {
                let xmag = orthographic.xmag();
                let ymag = orthographic.ymag();
                let orthographic_projection: OrthographicProjection = OrthographicProjection {
                    left: -xmag,
                    right: xmag,
                    top: ymag,
                    bottom: -ymag,
                    far: orthographic.zfar(),
                    near: orthographic.znear(),
                    ..Default::default()
                };

                node.insert(Camera {
                    projection_matrix: orthographic_projection.get_projection_matrix(),
                    ..Default::default()
                });
                node.insert(orthographic_projection).insert(Camera2d);
            }
            gltf::camera::Projection::Perspective(perspective) => {
                let mut perspective_projection: PerspectiveProjection = PerspectiveProjection {
                    fov: perspective.yfov(),
                    near: perspective.znear(),
                    ..Default::default()
                };
                if let Some(zfar) = perspective.zfar() {
                    perspective_projection.far = zfar;
                }
                if let Some(aspect_ratio) = perspective.aspect_ratio() {
                    perspective_projection.aspect_ratio = aspect_ratio;
                }
                node.insert(Camera {
                    projection_matrix: perspective_projection.get_projection_matrix(),
                    near: perspective_projection.near,
                    far: perspective_projection.far,
                    ..Default::default()
                });
                node.insert(perspective_projection);
                node.insert(Camera3d);
            }
        }
    }

    node.with_children(|parent| {
        if let Some(mesh) = gltf_node.mesh() {
            // append primitives
            for primitive in mesh.primitives() {
                let material = primitive.material();
                let material_label = material_label(&material);

                // This will make sure we load the default material now since it would not have been
                // added when iterating over all the gltf materials (since the default material is
                // not explicitly listed in the gltf).
                if !load_context.has_labeled_asset(&material_label) {
                    load_material(&material, load_context);
                }

                let primitive_label = primitive_label(&mesh, &primitive);
                let mesh_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                let material_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&material_label));

                let bounds = primitive.bounding_box();
                parent
                    .spawn_bundle(PbrBundle {
                        mesh: load_context.get_handle(mesh_asset_path),
                        material: load_context.get_handle(material_asset_path),
                        ..Default::default()
                    })
                    .insert(Aabb::from_min_max(
                        Vec3::from_slice(&bounds.min),
                        Vec3::from_slice(&bounds.max),
                    ));
            }
        }

        if let Some(light) = gltf_node.light() {
            match light.kind() {
                gltf::khr_lights_punctual::Kind::Directional => {
                    let mut entity = parent.spawn_bundle(DirectionalLightBundle {
                        directional_light: DirectionalLight {
                            color: Color::from(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for directional
                            // lights in lux (lm/m^2) which is what we need.
                            illuminance: light.intensity(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    if let Some(name) = light.name() {
                        entity.insert(Name::new(name.to_string()));
                    }
                }
                gltf::khr_lights_punctual::Kind::Point => {
                    let mut entity = parent.spawn_bundle(PointLightBundle {
                        point_light: PointLight {
                            color: Color::from(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for point lights in
                            // candela (lm/sr) which is luminous intensity and we need luminous power.
                            // For a point light, luminous power = 4 * pi * luminous intensity
                            intensity: light.intensity() * std::f32::consts::PI * 4.0,
                            range: light.range().unwrap_or(20.0),
                            radius: light.range().unwrap_or(0.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    if let Some(name) = light.name() {
                        entity.insert(Name::new(name.to_string()));
                    }
                }
                gltf::khr_lights_punctual::Kind::Spot {
                    inner_cone_angle: _inner_cone_angle,
                    outer_cone_angle: _outer_cone_angle,
                } => warn!("Spot lights are not yet supported."),
            }
        }

        // append other nodes
        for child in gltf_node.children() {
            if let Err(err) = load_node(&child, parent, load_context, buffer_data, animated_nodes) {
                gltf_error = Some(err);
                return;
            }
        }
    });
    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Returns the label for the `mesh`.
fn mesh_label(mesh: &gltf::Mesh) -> String {
    format!("Mesh{}", mesh.index())
}

/// Returns the label for the `mesh` and `primitive`.
fn primitive_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!("Mesh{}/Primitive{}", mesh.index(), primitive.index())
}

/// Returns the label for the `material`.
fn material_label(material: &gltf::Material) -> String {
    if let Some(index) = material.index() {
        format!("Material{}", index)
    } else {
        "MaterialDefault".to_string()
    }
}

/// Returns the label for the `texture`.
fn texture_label(texture: &gltf::Texture) -> String {
    format!("Texture{}", texture.index())
}

/// Returns the label for the `node`.
fn node_label(node: &gltf::Node) -> String {
    format!("Node{}", node.index())
}

/// Returns the label for the `scene`.
fn scene_label(scene: &gltf::Scene) -> String {
    format!("Scene{}", scene.index())
}

/// Extracts the texture sampler data from the glTF texture.
fn texture_sampler<'a>(texture: &gltf::Texture) -> SamplerDescriptor<'a> {
    let gltf_sampler = texture.sampler();

    SamplerDescriptor {
        address_mode_u: texture_address_mode(&gltf_sampler.wrap_s()),
        address_mode_v: texture_address_mode(&gltf_sampler.wrap_t()),

        mag_filter: gltf_sampler
            .mag_filter()
            .map(|mf| match mf {
                MagFilter::Nearest => FilterMode::Nearest,
                MagFilter::Linear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().mag_filter),

        min_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::NearestMipmapNearest
                | MinFilter::NearestMipmapLinear => FilterMode::Nearest,
                MinFilter::Linear
                | MinFilter::LinearMipmapNearest
                | MinFilter::LinearMipmapLinear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().min_filter),

        mipmap_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::Linear
                | MinFilter::NearestMipmapNearest
                | MinFilter::LinearMipmapNearest => FilterMode::Nearest,
                MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                    FilterMode::Linear
                }
            })
            .unwrap_or(SamplerDescriptor::default().mipmap_filter),

        ..Default::default()
    }
}

/// Maps the texture address mode form glTF to wgpu.
fn texture_address_mode(gltf_address_mode: &gltf::texture::WrappingMode) -> AddressMode {
    match gltf_address_mode {
        WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
        WrappingMode::Repeat => AddressMode::Repeat,
        WrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
    }
}

/// Maps the `primitive_topology` form glTF to `wgpu`.
fn get_primitive_topology(mode: Mode) -> Result<PrimitiveTopology, GltfError> {
    match mode {
        Mode::Points => Ok(PrimitiveTopology::PointList),
        Mode::Lines => Ok(PrimitiveTopology::LineList),
        Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
        Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
        Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
        mode => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}

fn alpha_mode(material: &Material) -> AlphaMode {
    match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}

/// Loads the raw glTF buffer data for a specific glTF file.
async fn load_buffers(
    gltf: &gltf::Gltf,
    load_context: &LoadContext<'_>,
    asset_path: &Path,
) -> Result<Vec<Vec<u8>>, GltfError> {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
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
                        let buffer_path = asset_path.parent().unwrap().join(uri);
                        let buffer_bytes = load_context.read_asset_bytes(buffer_path).await?;
                        buffer_bytes
                    }
                };
                buffer_data.push(buffer_bytes);
            }
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob);
                }
            }
        }
    }

    Ok(buffer_data)
}

fn resolve_node_hierarchy(
    nodes_intermediate: Vec<(String, GltfNode, Vec<usize>)>,
    asset_path: &Path,
) -> Vec<(String, GltfNode)> {
    let mut has_errored = false;
    let mut empty_children = VecDeque::new();
    let mut parents = vec![None; nodes_intermediate.len()];
    let mut unprocessed_nodes = nodes_intermediate
        .into_iter()
        .enumerate()
        .map(|(i, (label, node, children))| {
            for child in &children {
                if let Some(parent) = parents.get_mut(*child) {
                    *parent = Some(i);
                } else if !has_errored {
                    has_errored = true;
                    warn!("Unexpected child in GLTF Mesh {}", child);
                }
            }
            let children = children.into_iter().collect::<HashSet<_>>();
            if children.is_empty() {
                empty_children.push_back(i);
            }
            (i, (label, node, children))
        })
        .collect::<HashMap<_, _>>();
    let mut nodes = std::collections::HashMap::<usize, (String, GltfNode)>::new();
    while let Some(index) = empty_children.pop_front() {
        let (label, node, children) = unprocessed_nodes.remove(&index).unwrap();
        assert!(children.is_empty());
        nodes.insert(index, (label, node));
        if let Some(parent_index) = parents[index] {
            let (_, parent_node, parent_children) =
                unprocessed_nodes.get_mut(&parent_index).unwrap();

            assert!(parent_children.remove(&index));
            if let Some((_, child_node)) = nodes.get(&index) {
                parent_node.children.push(child_node.clone());
            }
            if parent_children.is_empty() {
                empty_children.push_back(parent_index);
            }
        }
    }
    if !unprocessed_nodes.is_empty() {
        warn!("GLTF model must be a tree: {:?}", asset_path);
    }
    let mut nodes_to_sort = nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect()
}

struct DataUri<'a> {
    mime_type: &'a str,
    base64: bool,
    data: &'a str,
}

fn split_once(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, delimiter);
    Some((iter.next()?, iter.next()?))
}

impl<'a> DataUri<'a> {
    fn parse(uri: &'a str) -> Result<DataUri<'a>, ()> {
        let uri = uri.strip_prefix("data:").ok_or(())?;
        let (mime_type, data) = split_once(uri, ',').ok_or(())?;

        let (mime_type, base64) = match mime_type.strip_suffix(";base64") {
            Some(mime_type) => (mime_type, true),
            None => (mime_type, false),
        };

        Ok(DataUri {
            mime_type,
            base64,
            data,
        })
    }

    fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        if self.base64 {
            base64::decode(self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::resolve_node_hierarchy;
    use crate::GltfNode;

    impl GltfNode {
        fn empty() -> Self {
            GltfNode {
                children: vec![],
                mesh: None,
                transform: bevy_transform::prelude::Transform::identity(),
            }
        }
    }
    #[test]
    fn node_hierarchy_single_node() {
        let result = resolve_node_hierarchy(
            vec![("l1".to_string(), GltfNode::empty(), vec![])],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_no_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 0);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_simple_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 1);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_hierarchy() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![2]),
                ("l3".to_string(), GltfNode::empty(), vec![3, 4, 5]),
                ("l4".to_string(), GltfNode::empty(), vec![6]),
                ("l5".to_string(), GltfNode::empty(), vec![]),
                ("l6".to_string(), GltfNode::empty(), vec![]),
                ("l7".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 7);
        assert_eq!(result[0].0, "l1");
        assert_eq!(result[0].1.children.len(), 1);
        assert_eq!(result[1].0, "l2");
        assert_eq!(result[1].1.children.len(), 1);
        assert_eq!(result[2].0, "l3");
        assert_eq!(result[2].1.children.len(), 3);
        assert_eq!(result[3].0, "l4");
        assert_eq!(result[3].1.children.len(), 1);
        assert_eq!(result[4].0, "l5");
        assert_eq!(result[4].1.children.len(), 0);
        assert_eq!(result[5].0, "l6");
        assert_eq!(result[5].1.children.len(), 0);
        assert_eq!(result[6].0, "l7");
        assert_eq!(result[6].1.children.len(), 0);
    }

    #[test]
    fn node_hierarchy_cyclic() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![1]),
                ("l2".to_string(), GltfNode::empty(), vec![0]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 0);
    }

    #[test]
    fn node_hierarchy_missing_node() {
        let result = resolve_node_hierarchy(
            vec![
                ("l1".to_string(), GltfNode::empty(), vec![2]),
                ("l2".to_string(), GltfNode::empty(), vec![]),
            ],
            PathBuf::new().as_path(),
        );

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "l2");
        assert_eq!(result[0].1.children.len(), 0);
    }
}
