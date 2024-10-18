use gltf::{Document, Node};

use bevy_asset::{Handle, LoadContext};
use bevy_color::Color;
use bevy_core::Name;
use bevy_core_pipeline::prelude::Camera3d;
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_hierarchy::{BuildChildren, ChildBuild, WorldChildBuilder};
use bevy_math::{Mat4, Vec3};
use bevy_pbr::{DirectionalLight, MeshMaterial3d, PointLight, SpotLight, StandardMaterial};
use bevy_render::{
    camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode},
    mesh::{
        morph::{MeshMorphWeights, MorphWeights},
        skinning::SkinnedMeshInverseBindposes,
        Mesh3d,
    },
    primitives::Aabb,
    view::Visibility,
};
use bevy_transform::components::Transform;
use bevy_utils::HashMap;
#[cfg(feature = "bevy_animation")]
use {
    bevy_animation::{AnimationTarget, AnimationTargetId},
    bevy_utils::HashSet,
    smallvec::SmallVec,
};

use crate::{
    GltfAssetLabel, GltfExtras, GltfMaterialExtras, GltfMaterialName, GltfMesh, GltfMeshExtras,
    GltfNode, GltfSkin,
};

#[cfg(feature = "bevy_animation")]
use super::AnimationContext;
use super::{GltfError, GltfLoaderSettings, GltfTreeIterator};

#[allow(clippy::result_large_err)]
pub fn load_nodes_and_skins(
    load_context: &mut LoadContext,
    gltf: &gltf::Gltf,
    meshes: &[Handle<GltfMesh>],
    #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    skinned_mesh_inverse_bindposes: &[Handle<SkinnedMeshInverseBindposes>],
) -> Result<
    (
        Vec<Handle<GltfNode>>,
        HashMap<Box<str>, Handle<GltfNode>>,
        Vec<Handle<GltfSkin>>,
        HashMap<Box<str>, Handle<GltfSkin>>,
    ),
    GltfError,
> {
    let mut unsorted_nodes = HashMap::<usize, Handle<GltfNode>>::new();
    let mut named_nodes = HashMap::new();
    let mut skins = vec![];
    let mut named_skins = HashMap::default();
    for node in GltfTreeIterator::try_new(gltf)? {
        let skin = node.skin().map(|skin| {
            let joints = skin
                .joints()
                .map(|joint| unsorted_nodes.get(&joint.index()).unwrap().clone())
                .collect();

            let gltf_skin = GltfSkin::new(
                &skin,
                joints,
                skinned_mesh_inverse_bindposes[skin.index()].clone(),
                super::extras::get_gltf_extras(skin.extras()),
            );

            let handle = load_context.add_labeled_asset(skin_label(&skin), gltf_skin);

            skins.push(handle.clone());
            if let Some(name) = skin.name() {
                named_skins.insert(name.into(), handle.clone());
            }

            handle
        });

        let children = node
            .children()
            .map(|child| unsorted_nodes.get(&child.index()).unwrap().clone())
            .collect();

        let mesh = node
            .mesh()
            .map(|mesh| mesh.index())
            .and_then(|i| meshes.get(i).cloned());

        let gltf_node = GltfNode::new(
            &node,
            children,
            mesh,
            node_transform(&node),
            skin,
            super::extras::get_gltf_extras(node.extras()),
        );

        #[cfg(feature = "bevy_animation")]
        let gltf_node = gltf_node.with_animation_root(animation_roots.contains(&node.index()));

        let handle = load_context.add_labeled_asset(gltf_node.asset_label().to_string(), gltf_node);
        unsorted_nodes.insert(node.index(), handle.clone());
        if let Some(name) = node.name() {
            named_nodes.insert(name.into(), handle);
        }
    }

    let mut nodes_to_sort = unsorted_nodes.into_iter().collect::<Vec<_>>();
    nodes_to_sort.sort_by_key(|(i, _)| *i);
    let nodes = nodes_to_sort
        .into_iter()
        .map(|(_, resolved)| resolved)
        .collect();

    Ok((nodes, named_nodes, skins, named_skins))
}

/// Loads a glTF node.
#[allow(clippy::too_many_arguments, clippy::result_large_err)]
pub fn load_node(
    gltf_node: &Node,
    world_builder: &mut WorldChildBuilder,
    root_load_context: &LoadContext,
    load_context: &mut LoadContext,
    settings: &GltfLoaderSettings,
    node_index_to_entity_map: &mut HashMap<usize, Entity>,
    entity_to_skin_index_map: &mut EntityHashMap<usize>,
    active_camera_found: &mut bool,
    parent_transform: &Transform,
    #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    #[cfg(feature = "bevy_animation")] mut animation_context: Option<AnimationContext>,
    document: &Document,
) -> Result<(), GltfError> {
    let mut gltf_error = None;
    let transform = node_transform(gltf_node);
    let world_transform = *parent_transform * transform;
    // according to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#instantiation,
    // if the determinant of the transform is negative we must invert the winding order of
    // triangles in meshes on the node.
    // instead we equivalently test if the global scale is inverted by checking if the number
    // of negative scale factors is odd. if so we will assign a copy of the material with face
    // culling inverted, rather than modifying the mesh data directly.
    let is_scale_inverted = world_transform.scale.is_negative_bitmask().count_ones() & 1 == 1;
    let mut node = world_builder.spawn((transform, Visibility::default()));

    let name = node_name(gltf_node);
    node.insert(name.clone());

    #[cfg(feature = "bevy_animation")]
    if animation_context.is_none() && animation_roots.contains(&gltf_node.index()) {
        // This is an animation root. Make a new animation context.
        animation_context = Some(AnimationContext {
            root: node.id(),
            path: SmallVec::new(),
        });
    }

    #[cfg(feature = "bevy_animation")]
    if let Some(ref mut animation_context) = animation_context {
        animation_context.path.push(name);

        node.insert(AnimationTarget {
            id: AnimationTargetId::from_names(animation_context.path.iter()),
            player: animation_context.root,
        });
    }

    if let Some(extras) = gltf_node.extras() {
        node.insert(GltfExtras {
            value: extras.get().to_string(),
        });
    }

    // create camera node
    if settings.load_cameras {
        if let Some(camera) = gltf_node.camera() {
            let projection = match camera.projection() {
                gltf::camera::Projection::Orthographic(orthographic) => {
                    let xmag = orthographic.xmag();
                    let orthographic_projection = OrthographicProjection {
                        near: orthographic.znear(),
                        far: orthographic.zfar(),
                        scaling_mode: ScalingMode::FixedHorizontal {
                            viewport_width: xmag,
                        },
                        ..OrthographicProjection::default_3d()
                    };

                    Projection::Orthographic(orthographic_projection)
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
                    Projection::Perspective(perspective_projection)
                }
            };
            node.insert((
                Camera3d::default(),
                projection,
                transform,
                Camera {
                    is_active: !*active_camera_found,
                    ..Default::default()
                },
            ));

            *active_camera_found = true;
        }
    }

    // Map node index to entity
    node_index_to_entity_map.insert(gltf_node.index(), node.id());

    let mut morph_weights = None;

    node.with_children(|parent| {
        // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
        if !settings.load_meshes.is_empty() {
            if let Some(mesh) = gltf_node.mesh() {
                // append primitives
                for primitive in mesh.primitives() {
                    let material = primitive.material();
                    let material_label =
                        super::material::material_label(&material, is_scale_inverted);

                    // This will make sure we load the default material now since it would not have been
                    // added when iterating over all the gltf materials (since the default material is
                    // not explicitly listed in the gltf).
                    // It also ensures an inverted scale copy is instantiated if required.
                    if !root_load_context.has_labeled_asset(&material_label)
                        && !load_context.has_labeled_asset(&material_label)
                    {
                        super::material::load_material(
                            &material,
                            load_context,
                            document,
                            is_scale_inverted,
                        );
                    }

                    let primitive_label = GltfAssetLabel::Primitive {
                        mesh: mesh.index(),
                        primitive: primitive.index(),
                    };
                    let bounds = primitive.bounding_box();

                    let mut mesh_entity = parent.spawn((
                        // TODO: handle missing label handle errors here?
                        Mesh3d(load_context.get_label_handle(primitive_label.to_string())),
                        MeshMaterial3d::<StandardMaterial>(
                            load_context.get_label_handle(&material_label),
                        ),
                    ));

                    let target_count = primitive.morph_targets().len();
                    if target_count != 0 {
                        let weights = match mesh.weights() {
                            Some(weights) => weights.to_vec(),
                            None => vec![0.0; target_count],
                        };

                        if morph_weights.is_none() {
                            morph_weights = Some(weights.clone());
                        }

                        // unwrap: the parent's call to `MeshMorphWeights::new`
                        // means this code doesn't run if it returns an `Err`.
                        // According to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets
                        // they should all have the same length.
                        // > All morph target accessors MUST have the same count as
                        // > the accessors of the original primitive.
                        mesh_entity.insert(MeshMorphWeights::new(weights).unwrap());
                    }
                    mesh_entity.insert(Aabb::from_min_max(
                        Vec3::from_slice(&bounds.min),
                        Vec3::from_slice(&bounds.max),
                    ));

                    if let Some(extras) = primitive.extras() {
                        mesh_entity.insert(GltfExtras {
                            value: extras.get().to_string(),
                        });
                    }

                    if let Some(extras) = mesh.extras() {
                        mesh_entity.insert(GltfMeshExtras {
                            value: extras.get().to_string(),
                        });
                    }

                    if let Some(extras) = material.extras() {
                        mesh_entity.insert(GltfMaterialExtras {
                            value: extras.get().to_string(),
                        });
                    }

                    if let Some(name) = material.name() {
                        mesh_entity.insert(GltfMaterialName(String::from(name)));
                    }

                    mesh_entity.insert(Name::new(super::mesh::primitive_name(&mesh, &primitive)));
                    // Mark for adding skinned mesh
                    if let Some(skin) = gltf_node.skin() {
                        entity_to_skin_index_map.insert(mesh_entity.id(), skin.index());
                    }
                }
            }
        }

        if settings.load_lights {
            if let Some(light) = gltf_node.light() {
                match light.kind() {
                    gltf::khr_lights_punctual::Kind::Directional => {
                        let mut entity = parent.spawn(DirectionalLight {
                            color: Color::srgb_from_array(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for directional
                            // lights in lux (lm/m^2) which is what we need.
                            illuminance: light.intensity(),
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                    gltf::khr_lights_punctual::Kind::Point => {
                        let mut entity = parent.spawn(PointLight {
                            color: Color::srgb_from_array(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for point lights in
                            // candela (lm/sr) which is luminous intensity and we need luminous power.
                            // For a point light, luminous power = 4 * pi * luminous intensity
                            intensity: light.intensity() * core::f32::consts::PI * 4.0,
                            range: light.range().unwrap_or(20.0),
                            radius: 0.0,
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                    gltf::khr_lights_punctual::Kind::Spot {
                        inner_cone_angle,
                        outer_cone_angle,
                    } => {
                        let mut entity = parent.spawn(SpotLight {
                            color: Color::srgb_from_array(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for spot lights in
                            // candela (lm/sr) which is luminous intensity and we need luminous power.
                            // For a spot light, we map luminous power = 4 * pi * luminous intensity
                            intensity: light.intensity() * core::f32::consts::PI * 4.0,
                            range: light.range().unwrap_or(20.0),
                            radius: light.range().unwrap_or(0.0),
                            inner_angle: inner_cone_angle,
                            outer_angle: outer_cone_angle,
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            entity.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras() {
                            entity.insert(GltfExtras {
                                value: extras.get().to_string(),
                            });
                        }
                    }
                }
            }
        }

        // append other nodes
        for child in gltf_node.children() {
            if let Err(err) = load_node(
                &child,
                parent,
                root_load_context,
                load_context,
                settings,
                node_index_to_entity_map,
                entity_to_skin_index_map,
                active_camera_found,
                &world_transform,
                #[cfg(feature = "bevy_animation")]
                animation_roots,
                #[cfg(feature = "bevy_animation")]
                animation_context.clone(),
                document,
            ) {
                gltf_error = Some(err);
                return;
            }
        }
    });

    // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
    if !settings.load_meshes.is_empty() {
        if let (Some(mesh), Some(weights)) = (gltf_node.mesh(), morph_weights) {
            let primitive_label = mesh.primitives().next().map(|p| GltfAssetLabel::Primitive {
                mesh: mesh.index(),
                primitive: p.index(),
            });
            let first_mesh =
                primitive_label.map(|label| load_context.get_label_handle(label.to_string()));
            node.insert(MorphWeights::new(weights, first_mesh)?);
        }
    }

    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Calculate the transform of gLTF node.
///
/// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
/// on [`Node::transform()`] directly because it uses optimized glam types and
/// if `libm` feature of `bevy_math` crate is enabled also handles cross
/// platform determinism properly.
pub fn node_transform(node: &Node) -> Transform {
    match node.transform() {
        gltf::scene::Transform::Matrix { matrix } => {
            Transform::from_matrix(Mat4::from_cols_array_2d(&matrix))
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Transform {
            translation: Vec3::from(translation),
            rotation: bevy_math::Quat::from_array(rotation),
            scale: Vec3::from(scale),
        },
    }
}

pub fn node_name(node: &Node) -> Name {
    let name = node
        .name()
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("GltfNode{}", node.index()));
    Name::new(name)
}

/// Return the label for the `skin`.
fn skin_label(skin: &gltf::Skin) -> String {
    GltfAssetLabel::Skin(skin.index()).to_string()
}
