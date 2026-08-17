//! A `bevy_editor_cam` extension that provides a skybox rendered by a different camera with a
//! different field of view than the camera it is added to. This allows you to use very narrow
//! camera FOV, or even orthographic projections, while keeping the appearance of the skybox
//! unchanged.
//!
//! To use it, add a [`IndependentSkybox`] component to a camera,
//! and add the [`IndependentSkyboxPlugin`] to your app.

use bevy_app::prelude::*;
use bevy_asset::Handle;
use bevy_camera::{prelude::*, visibility::RenderLayers, Hdr};
use bevy_core_pipeline::Skybox;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::Quat;
use bevy_render::view::Msaa;
use bevy_transform::prelude::*;

/// See the [module](self) docs.
pub struct IndependentSkyboxPlugin;

impl Plugin for IndependentSkyboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                IndependentSkyboxCamera::spawn,
                IndependentSkyboxCamera::despawn,
                ApplyDeferred,
                IndependentSkyboxCamera::update,
            )
                .chain(),
        );
    }
}

/// When working with orthographic or very narrow field of view cameras, the skybox can appear
/// distorted. This component allows you to add a skybox to a camera that is rendered by
/// a separate camera with its own field of view.
///
/// There are two camera entities involved when using this setup:
///
/// 1. The camera you add the [`IndependentSkybox`] component to. This should be a camera controller, and can use any
///    projection you like.
/// 2. A separate camera that is created automatically to render the skybox. This camera will
///    copy the position and orientation of the first camera, but will use its own field of
///    view, as specified by the [`IndependentSkybox::fov`] setting. This camera will have the
///    [`IndependentSkyboxCamera`] component.
///
/// This struct controls the parameters used to render the skybox.
/// [`IndependentSkyboxPlugin`] adds systems to manage the lifecycle of the skybox camera
/// automatically. You just need to add this component to a camera, and a separate linked skybox camera
/// will be created, updated, and destroyed as needed.
#[derive(Debug, Clone, Component)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct IndependentSkybox {
    /// The image to render as a skybox.
    pub skybox: Handle<Image>,
    /// Used to set [`Skybox::brightness`].
    pub brightness: f32,
    /// Used to set [`Skybox::rotation`].
    pub rotation: Quat,
    /// The [`Camera::order`] of the skybox camera, offset from the camera it is tracking. This
    /// should be lower than the order of the primary camera controller camera. the default value
    /// should be sufficient for most cases. You can override this if you have a more complex use
    /// case with multiple cameras.
    pub skybox_cam_order_offset: isize,
    /// The field of view of the skybox.
    pub fov: SkyboxFov,
    /// The corresponding skybox camera entity.
    skybox_cam: Option<Entity>,
}

impl IndependentSkybox {
    /// Create a new [`IndependentSkybox`] with default settings and the provided skybox image.
    pub fn new(skybox: Handle<Image>, brightness: f32, rotation: Quat) -> Self {
        Self {
            skybox,
            brightness,
            rotation,
            ..Default::default()
        }
    }
}

impl Default for IndependentSkybox {
    fn default() -> Self {
        Self {
            skybox: Default::default(),
            brightness: 500.0,
            rotation: Quat::IDENTITY,
            skybox_cam_order_offset: -1_000,
            fov: Default::default(),
            skybox_cam: Default::default(),
        }
    }
}

/// Field of view setting for the [`IndependentSkybox`]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub enum SkyboxFov {
    /// Match the [`PerspectiveProjection::fov`] of the camera this skybox camera is following.
    Auto,
    /// Use a fixed value for the skybox field of view. This value is equivalent to
    /// [`PerspectiveProjection::fov`].
    Fixed(f32),
}

impl Default for SkyboxFov {
    fn default() -> Self {
        Self::Fixed(PerspectiveProjection::default().fov)
    }
}

/// Used to track the camera that is used to render a skybox, using the [`IndependentSkybox`]
/// component settings placed on a camera.
#[derive(Component)]
pub struct IndependentSkyboxCamera {
    /// The camera that this skybox camera is observing.
    driven_by: Entity,
}

impl IndependentSkyboxCamera {
    /// Spawns [`IndependentSkyboxCamera`]s when a [`IndependentSkybox`] exists without a skybox
    /// entity.
    pub fn spawn(
        mut commands: Commands,
        mut editor_cams: Query<(Entity, &mut IndependentSkybox, &mut Camera, &Msaa)>,
        skybox_cams: Query<&IndependentSkyboxCamera>,
    ) {
        for (editor_cam_entity, mut editor_without_skybox, mut camera, msaa) in
            editor_cams.iter_mut().filter(|(_, config, ..)| {
                config
                    .skybox_cam
                    .and_then(|e| skybox_cams.get(e).ok())
                    .is_none()
            })
        {
            camera.clear_color = ClearColorConfig::None;
            commands.entity(editor_cam_entity).insert(Hdr);

            let entity = commands
                .spawn((
                    Camera3d::default(),
                    Hdr,
                    Camera {
                        order: camera.order + editor_without_skybox.skybox_cam_order_offset,
                        clear_color: ClearColorConfig::None,
                        ..Default::default()
                    },
                    Projection::Perspective(PerspectiveProjection {
                        fov: match editor_without_skybox.fov {
                            SkyboxFov::Auto => PerspectiveProjection::default().fov,
                            SkyboxFov::Fixed(fov) => fov,
                        },
                        ..Default::default()
                    }),
                    RenderLayers::none(),
                    Skybox {
                        image: Some(editor_without_skybox.skybox.clone()),
                        brightness: editor_without_skybox.brightness,
                        rotation: editor_without_skybox.rotation,
                    },
                    IndependentSkyboxCamera {
                        driven_by: editor_cam_entity,
                    },
                    *msaa,
                ))
                .id();
            editor_without_skybox.skybox_cam = Some(entity);
        }
    }

    /// Despawns [`IndependentSkyboxCamera`]s when their corresponding [`IndependentSkybox`] entity
    /// does not exist.
    pub fn despawn(
        mut commands: Commands,
        skybox_cams: Query<(Entity, &IndependentSkyboxCamera)>,
        editor_cams: Query<&IndependentSkybox>,
    ) {
        for (skybox_entity, skybox) in &skybox_cams {
            if editor_cams.get(skybox.driven_by).is_err() {
                commands.entity(skybox_entity).despawn();
            }
        }
    }

    /// Update the position and projection of this [`IndependentSkyboxCamera`] to copy the camera it
    /// is following.
    pub fn update(
        mut editor_cams: Query<
            (&IndependentSkybox, &Transform, &Projection, &Camera),
            (
                Or<(Changed<IndependentSkybox>, Changed<Transform>)>,
                Without<Self>,
            ),
        >,
        mut skybox_cams: Query<(Mut<Transform>, Mut<Projection>, Mut<Camera>), With<Self>>,
    ) {
        for (editor_cam, editor_transform, editor_projection, camera) in &mut editor_cams {
            let Some(skybox_entity) = editor_cam.skybox_cam else {
                continue;
            };
            let Ok((mut skybox_transform, mut skybox_projection, mut skybox_camera)) =
                skybox_cams.get_mut(skybox_entity)
            else {
                continue;
            };

            skybox_camera.viewport.clone_from(&camera.viewport);

            if let Projection::Perspective(editor_perspective) = editor_projection {
                *skybox_projection = Projection::Perspective(PerspectiveProjection {
                    fov: match editor_cam.fov {
                        SkyboxFov::Auto => editor_perspective.fov,
                        SkyboxFov::Fixed(fov) => fov,
                    },
                    ..editor_perspective.clone()
                });
            }

            *skybox_transform = *editor_transform;
        }
    }
}
