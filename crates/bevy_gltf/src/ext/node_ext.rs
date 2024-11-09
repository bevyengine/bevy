use gltf::Node;
#[cfg(feature = "bevy_animation")]
use smallvec::SmallVec;

#[cfg(feature = "bevy_animation")]
use bevy_animation::{AnimationTarget, AnimationTargetId};
use bevy_asset::{Handle, LoadContext};
use bevy_color::Color;
use bevy_core::Name;
use bevy_core_pipeline::core_3d::Camera3d;
use bevy_ecs::{
    entity::{Entity, EntityHashMap},
    world::World,
};
use bevy_hierarchy::BuildChildren;
use bevy_math::{Mat4, Vec3};
use bevy_pbr::{DirectionalLight, PointLight, SpotLight};
use bevy_render::{
    camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode},
    mesh::morph::MorphWeights,
    view::Visibility,
};
use bevy_transform::components::Transform;
use bevy_utils::{HashMap, HashSet};

use crate::{GltfAssetLabel, GltfError, GltfLoaderSettings, GltfNode};

use super::{ExtrasExt, MeshExt, PrimitiveExt, SkinExt};

/// [`Node`] extension
pub trait NodeExt {
    fn load_node(
        &self,
        load_context: &mut LoadContext,
        unsorted_nodes: &mut HashMap<usize, Handle<GltfNode>>,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    ) -> GltfNode;

    /// Loads a glTF node.
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    fn load_scene_node(
        &self,
        world: &mut World,
        root_load_context: &LoadContext,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        node_index_to_entity_map: &mut HashMap<usize, Entity>,
        entity_to_skin_index_map: &mut EntityHashMap<usize>,
        active_camera_found: &mut bool,
        parent_transform: &Transform,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
        #[cfg(feature = "bevy_animation")] animation_context: Option<AnimationContext>,
        document: &gltf::Document,
    ) -> Result<Entity, GltfError>;

    /// Calculate the transform of gLTF node.
    ///
    /// This should be used instead of calling [`gltf::scene::Transform::matrix()`]
    /// on [`Node::transform()`] directly because it uses optimized glam types and
    /// if `libm` feature of `bevy_math` crate is enabled also handles cross
    /// platform determinism properly.
    fn node_transform(&self) -> Transform;

    /// Create a [`GltfAssetLabel`] for the [`Node`]
    fn to_label(&self) -> GltfAssetLabel;

    /// Create a [`Name`] for the [`Node`]
    fn to_name(&self) -> Name;

    /// Check if node is skinned
    fn is_skinned(&self) -> bool;

    /// Get index of [`Mesh`](gltf::Mesh) on [`Node`]
    fn mesh_index(&self) -> Option<usize>;

    fn paths_recur(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    );
}

impl NodeExt for Node<'_> {
    fn load_node(
        &self,
        load_context: &mut LoadContext,
        unsorted_nodes: &mut HashMap<usize, Handle<GltfNode>>,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
    ) -> GltfNode {
        let skin = self
            .skin()
            .map(|skin| load_context.get_label_handle(skin.to_label().to_string()));

        let children = self
            .children()
            .map(|child| unsorted_nodes.get(&child.index()).unwrap().clone())
            .collect();

        let mesh = self
            .mesh()
            .map(|mesh| load_context.get_label_handle(mesh.to_label().to_string()));

        let gltf_node = GltfNode::new(
            self,
            children,
            mesh,
            self.node_transform(),
            skin,
            self.extras().get(),
        );

        #[cfg(feature = "bevy_animation")]
        let gltf_node = gltf_node.with_animation_root(animation_roots.contains(&self.index()));

        gltf_node
    }

    /// Loads a glTF node.
    #[allow(clippy::too_many_arguments, clippy::result_large_err)]
    fn load_scene_node(
        &self,
        world: &mut World,
        root_load_context: &LoadContext,
        load_context: &mut LoadContext,
        settings: &GltfLoaderSettings,
        node_index_to_entity_map: &mut HashMap<usize, Entity>,
        entity_to_skin_index_map: &mut EntityHashMap<usize>,
        active_camera_found: &mut bool,
        parent_transform: &Transform,
        #[cfg(feature = "bevy_animation")] animation_roots: &HashSet<usize>,
        #[cfg(feature = "bevy_animation")] mut animation_context: Option<AnimationContext>,
        document: &gltf::Document,
    ) -> Result<Entity, GltfError> {
        let transform = self.node_transform();
        let world_transform = *parent_transform * transform;
        // according to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#instantiation,
        // if the determinant of the transform is negative we must invert the winding order of
        // triangles in meshes on the node.
        // instead we equivalently test if the global scale is inverted by checking if the number
        // of negative scale factors is odd. if so we will assign a copy of the material with face
        // culling inverted, rather than modifying the mesh data directly.
        let is_scale_inverted = world_transform.scale.is_negative_bitmask().count_ones() & 1 == 1;
        let mut node = world.spawn((transform, Visibility::default()));

        let name = self.to_name();
        node.insert(name.clone());

        #[cfg(feature = "bevy_animation")]
        if animation_context.is_none() && animation_roots.contains(&self.index()) {
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

        if let Some(extras) = self.extras().get() {
            node.insert(extras);
        }

        // create camera node
        if settings.load_cameras {
            if let Some(camera) = self.camera() {
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
                        let mut perspective_projection: PerspectiveProjection =
                            PerspectiveProjection {
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
        node_index_to_entity_map.insert(self.index(), node.id());

        let node = node.id();
        let mut morph_weights = None;

        // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
        if !settings.load_meshes.is_empty() {
            if let Some(mesh) = self.mesh() {
                // append primitives
                let node_primitives = mesh
                    .primitives()
                    .map(|primitive| {
                        primitive.load_scene_primitive(
                            world,
                            root_load_context,
                            load_context,
                            &mesh,
                            self.skin().as_ref(),
                            &mut morph_weights,
                            entity_to_skin_index_map,
                            is_scale_inverted,
                            document,
                        )
                    })
                    .collect::<Result<Vec<Entity>, GltfError>>()?;

                world.entity_mut(node).add_children(&node_primitives);
            }
        }

        if settings.load_lights {
            if let Some(light) = self.light() {
                match light.kind() {
                    gltf::khr_lights_punctual::Kind::Directional => {
                        let mut node_directional_light = world.spawn(DirectionalLight {
                            color: Color::srgb_from_array(light.color()),
                            // NOTE: KHR_punctual_lights defines the intensity units for directional
                            // lights in lux (lm/m^2) which is what we need.
                            illuminance: light.intensity(),
                            ..Default::default()
                        });
                        if let Some(name) = light.name() {
                            node_directional_light.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras().get() {
                            node_directional_light.insert(extras);
                        }
                        let node_directional_light = node_directional_light.id();

                        world.entity_mut(node).add_child(node_directional_light);
                    }
                    gltf::khr_lights_punctual::Kind::Point => {
                        let mut node_point_light = world.spawn(PointLight {
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
                            node_point_light.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras().get() {
                            node_point_light.insert(extras);
                        }
                        let node_point_light = node_point_light.id();

                        world.entity_mut(node).add_child(node_point_light);
                    }
                    gltf::khr_lights_punctual::Kind::Spot {
                        inner_cone_angle,
                        outer_cone_angle,
                    } => {
                        let mut node_spot_light = world.spawn(SpotLight {
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
                            node_spot_light.insert(Name::new(name.to_string()));
                        }
                        if let Some(extras) = light.extras().get() {
                            node_spot_light.insert(extras);
                        }
                        let node_spot_light = node_spot_light.id();

                        world.entity_mut(node).add_child(node_spot_light);
                    }
                }
            }
        }

        // append other nodes
        let children = self
            .children()
            .map(|child| {
                child.load_scene_node(
                    world,
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
                )
            })
            .collect::<Result<Vec<Entity>, GltfError>>()?;

        world.entity_mut(node).add_children(&children);

        // Only include meshes in the output if they're set to be retained in the MAIN_WORLD and/or RENDER_WORLD by the load_meshes flag
        if !settings.load_meshes.is_empty() {
            if let (Some(mesh), Some(weights)) = (self.mesh(), morph_weights) {
                let primitive_label = mesh.primitives().next().map(|p| GltfAssetLabel::Primitive {
                    mesh: mesh.index(),
                    primitive: p.index(),
                });
                let first_mesh =
                    primitive_label.map(|label| load_context.get_label_handle(label.to_string()));
                world
                    .entity_mut(node)
                    .insert(MorphWeights::new(weights, first_mesh)?);
            }
        }

        Ok(node)
    }

    fn node_transform(&self) -> Transform {
        match self.transform() {
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

    fn paths_recur(
        &self,
        current_path: &[Name],
        paths: &mut HashMap<usize, (usize, Vec<Name>)>,
        root_index: usize,
        visited: &mut HashSet<usize>,
    ) {
        let mut path = current_path.to_owned();
        path.push(self.to_name());
        visited.insert(self.index());
        for child in self.children() {
            if !visited.contains(&child.index()) {
                child.paths_recur(&path, paths, root_index, visited);
            }
        }
        paths.insert(self.index(), (root_index, path));
    }

    fn to_label(&self) -> GltfAssetLabel {
        GltfAssetLabel::Node(self.index())
    }

    fn to_name(&self) -> Name {
        let name = self
            .name()
            .map(ToString::to_string)
            .unwrap_or_else(|| format!("GltfNode{}", self.index()));
        Name::new(name)
    }

    fn is_skinned(&self) -> bool {
        self.skin().is_some()
    }

    fn mesh_index(&self) -> Option<usize> {
        self.mesh().map(|mesh_info| mesh_info.index())
    }
}

#[cfg(feature = "bevy_animation")]
#[derive(Clone)]
pub struct AnimationContext {
    // The nearest ancestor animation root.
    pub root: Entity,
    // The path to the animation root. This is used for constructing the
    // animation target UUIDs.
    pub path: SmallVec<[Name; 8]>,
}
