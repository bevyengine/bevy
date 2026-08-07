use crate::camera::NormalizedRenderTargetExt;
use crate::texture::ManualTextureViews;
use bevy_asset::Assets;
use bevy_camera::{Camera, RenderTarget, Viewport};
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::With;
use bevy_ecs::system::{Query, Res, Single};
use bevy_image::Image;
use bevy_log::warn_once;
use bevy_math::AspectRatio;
use bevy_window::{PrimaryWindow, Window};
use glam::UVec2;
use tracing::warn;

#[derive(Component, Clone, Debug)]
/// Configures how to box the camera output, if at all.
pub enum CameraBox {
    /// Keep the output at a static resolution, and box only if the actual resolution would be larger.
    /// If the output resolution is smaller than the desired output, it will output at the
    /// smaller resolution.
    StaticResolution { resolution: UVec2 },

    /// Keep the output at a specific Aspect Ratio. If the output resolution would not fit within
    /// the desired Aspect Ratio, box the resolution to force it to fit.
    StaticAspectRatio { aspect_ratio: AspectRatio },

    /// Static Letterboxing with a specific size for each of the bars.
    /// If the boxes are larger than the output, then letterboxing will be disabled.
    LetterBox { top: u32, bottom: u32 },

    /// Static Pillarboxing with a specific size for each of the bars.
    /// If the boxes are larger than the output, then pillarboxing will be disabled.
    PillarBox { left: u32, right: u32 },

    /// Static Window-boxing, with a specific size for bars on all sizes of the output.
    /// If the boxes are larger than the output, then window-boxing will be disabled.
    WindowBox {
        top: u32,
        bottom: u32,
        left: u32,
        right: u32,
    },
}

pub fn box_cameras(
    mut boxed_cameras: Query<(&mut Camera, &mut RenderTarget, &CameraBox)>,
    primary_window: Option<Single<Entity, With<PrimaryWindow>>>,
    windows: Query<(Entity, &Window)>,
    texture_views: Res<ManualTextureViews>,
    images: Res<Assets<Image>>,
) {
    let primary_window = primary_window.map(Single::into_inner);
    for (mut camera, target, camera_box) in boxed_cameras.iter_mut() {
        let target = match target
            .normalize(primary_window)
            .map(|t| t.get_render_target_info(windows, &images, &texture_views))
        {
            // As a note this failure case should be rare, such as render target being primary window
            // but no primary window exists.
            None => continue,
            Some(Err(e)) => {
                warn_once!("Missing Render Target Info: {:#?}", e);
                continue;
            }
            Some(Ok(target)) => target,
        };

        let mut viewport = match &mut camera.viewport {
            None => Viewport::default(),
            Some(vp) => vp.to_owned(),
        };

        let physical_size = &target.physical_size;
        match camera_box {
            CameraBox::StaticResolution { resolution } => {
                if &target.physical_size == resolution {
                    camera.viewport = None;
                    continue;
                }

                let clamped_resolution = if &viewport.physical_size != resolution {
                    resolution.clamp(UVec2::ONE, target.physical_size)
                } else {
                    *resolution
                };

                let render_placement = (target.physical_size
                    - resolution.clamp(UVec2::ZERO, target.physical_size))
                    / 2;

                viewport.physical_size = clamped_resolution;
                viewport.physical_position = render_placement;
                camera.viewport = Some(viewport);
            }
            CameraBox::StaticAspectRatio { aspect_ratio } => {
                let physical_aspect_ratio = match AspectRatio::try_from(physical_size.as_vec2()) {
                    Ok(ar) if ar.ratio() == aspect_ratio.ratio() => {
                        camera.viewport = None;
                        continue;
                    }
                    Err(e) => {
                        warn!(
                            "Error occurred in aspect ratio scaling calculation: {:?}",
                            e
                        );
                        continue;
                    }
                    Ok(ar) => ar,
                };
                let (render_resolution, render_position) = if physical_aspect_ratio.ratio()
                    > aspect_ratio.ratio()
                {
                    let render_height = physical_size.y;
                    let render_width = (render_height as f32 * aspect_ratio.ratio()).round() as u32;

                    (
                        UVec2::new(physical_size.x / 2 - render_width / 2, 0),
                        UVec2::new(render_width, render_height),
                    )
                } else {
                    let render_width = physical_size.x;
                    let render_height = (render_width as f32 / aspect_ratio.ratio()).round() as u32;
                    (
                        UVec2::new(0, physical_size.y / 2 - render_height / 2),
                        UVec2::new(render_width, render_height),
                    )
                };

                viewport.physical_size = render_resolution;
                viewport.physical_position = render_position;
                camera.viewport = Some(viewport);
            }
            CameraBox::LetterBox { top, bottom } => {
                let letterbox_height = top + bottom;

                let render_resolution =
                    UVec2::new(physical_size.x, physical_size.y - letterbox_height);

                let render_position = UVec2::new(0, *top);

                if render_resolution.y == 0
                    || !is_within_rect(physical_size, &render_position, &render_resolution)
                {
                    camera.viewport = None;
                    continue;
                }

                viewport.physical_size = render_resolution;
                viewport.physical_position = render_position;
                camera.viewport = Some(viewport);
            }
            CameraBox::PillarBox { left, right } => {
                let pillarbox_width = left + right;
                let render_resolution =
                    UVec2::new(physical_size.x - pillarbox_width, physical_size.y);

                let render_position = UVec2::new(*left, 0);
                if render_resolution.x == 0
                    || !is_within_rect(physical_size, &render_position, &render_resolution)
                {
                    camera.viewport = None;
                    continue;
                }

                viewport.physical_size = render_resolution; //output resolution
                viewport.physical_position = render_position; // resolution offset;
                camera.viewport = Some(viewport);
            }
            CameraBox::WindowBox {
                top,
                bottom,
                left,
                right,
            } => {
                let letterbox_height = top + bottom;
                let pillarbox_width = left + right;

                let render_resolution = UVec2::new(
                    physical_size.y - letterbox_height,
                    physical_size.x - pillarbox_width,
                );

                let render_position = UVec2::new(*left, *top);

                if render_resolution.x == 0
                    || !is_within_rect(physical_size, &render_position, &render_resolution)
                {
                    // No boxing
                    camera.viewport = None;
                    continue;
                }

                viewport.physical_size = render_resolution; //output resolution
                viewport.physical_position = render_position; // resolution offset;
                camera.viewport = Some(viewport);
            }
        }
    }
}

fn is_within_rect(rect: &UVec2, position: &UVec2, size: &UVec2) -> bool {
    let actual_bounds = position + size;
    rect.x >= actual_bounds.x && rect.y >= actual_bounds.y
}

#[cfg(test)]
mod tests {
    use super::*;
    
    mod systems {
        use bevy_app::{App, First};
        use super::*;
        use bevy_camera::RenderTarget;
        use bevy_window::{WindowRef, WindowResolution};
        use crate::camera::CameraPlugin;

        const W360P: UVec2 = UVec2::new(640, 360);
        const W720P: UVec2 = UVec2::new(1280, 720);
        const W180P: UVec2 = UVec2::new(320, 180);

        fn setup_app(camerabox: CameraBox, window_resolution: WindowResolution) -> (App, Entity) {
            let mut app = App::new();

            app.init_resource::<ManualTextureViews>();
            app.init_resource::<Assets<Image>>();
            app.add_systems(First, box_cameras);
            app.world_mut().spawn((
                Window {
                    resolution: window_resolution,
                    ..Window::default()
                },
                PrimaryWindow,
            ));
            let camera_id = app
                .world_mut()
                .spawn((
                    Camera {
                        viewport: None,
                        is_active: true,
                        ..Camera::default()
                    },
                    RenderTarget::Window(WindowRef::Primary),
                    camerabox,
                ))
                .id();
            (app, camera_id)
        }

        #[test]
        fn test_basic_windowboxing() {
            let (mut app, camera_id) = setup_app(
                CameraBox::WindowBox {
                    left: 10,
                    right: 10,
                    top: 10,
                    bottom: 10,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(10, 10));
            assert_eq!(viewport.physical_size, UVec2::new(620, 340));

            let (mut app, camera_id) = setup_app(
                CameraBox::WindowBox {
                    left: 650,
                    right: 0,
                    top: 370,
                    bottom: 0,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport;
            assert!(viewport.is_none());
        }

        #[test]
        fn test_basic_pillarboxing() {
            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 2,
                    right: 2,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(2, 0));
            assert_eq!(viewport.physical_size, UVec2::new(636, 360));

            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 5,
                    right: 0,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(5, 0));
            assert_eq!(viewport.physical_size, UVec2::new(635, 360));

            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 0,
                    right: 5,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 0));
            assert_eq!(viewport.physical_size, UVec2::new(635, 360));

            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 5,
                    right: 10,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(5, 0));
            assert_eq!(viewport.physical_size, UVec2::new(625, 360));

            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 10,
                    right: 5,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(10, 0));
            assert_eq!(viewport.physical_size, UVec2::new(625, 360));

            let (mut app, camera_id) = setup_app(
                CameraBox::PillarBox {
                    left: 640,
                    right: 0,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport;
            assert!(viewport.is_none());
        }

        #[test]
        fn test_basic_letterboxing() {
            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 2,
                    bottom: 2,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 2));
            assert_eq!(viewport.physical_size, UVec2::new(640, 356));

            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 5,
                    bottom: 0,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 5));
            assert_eq!(viewport.physical_size, UVec2::new(640, 355));

            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 0,
                    bottom: 5,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 0));
            assert_eq!(viewport.physical_size, UVec2::new(640, 355));

            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 10,
                    bottom: 5,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 10));
            assert_eq!(viewport.physical_size, UVec2::new(640, 345));

            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 5,
                    bottom: 10,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 5));
            assert_eq!(viewport.physical_size, UVec2::new(640, 345));

            let (mut app, camera_id) = setup_app(
                CameraBox::LetterBox {
                    top: 360,
                    bottom: 0,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport;
            assert!(viewport.is_none());

        }

        #[test]
        fn test_basic_resolution() {
            let (mut app, camera_id) = setup_app(
                CameraBox::StaticResolution {
                    resolution: W360P.into(),
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport;
            assert!(viewport.is_none());
            let (mut app, camera_id) = setup_app(
                CameraBox::StaticResolution {
                    resolution: W360P.into(),
                },
                W720P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(320, 180));
            assert_eq!(viewport.physical_size, W360P);

            let (mut app, camera_id) = setup_app(
                CameraBox::StaticResolution {
                    resolution: W360P.into(),
                },
                W180P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(0, 0));
            assert_eq!(viewport.physical_size, W180P);
        }

        #[test]
        fn test_basic_aspect_ratio() {
            let desired_aspect_ratio = AspectRatio::try_from(W720P.as_vec2()).unwrap();
            let (mut app, camera_id) = setup_app(
                CameraBox::StaticAspectRatio {
                    aspect_ratio: desired_aspect_ratio,
                },
                W360P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport;
            assert!(viewport.is_none());

            let desired_aspect_ratio = AspectRatio::try_new(640., 480.).unwrap();
            let (mut app, camera_id) = setup_app(
                CameraBox::StaticAspectRatio {
                    aspect_ratio: desired_aspect_ratio,
                },
                W720P.into(),
            );
            app.update();
            let viewport = app
                .world()
                .get::<Camera>(camera_id)
                .unwrap()
                .to_owned()
                .viewport
                .unwrap();
            assert_eq!(viewport.physical_position, UVec2::new(160, 0));
            assert_eq!(viewport.physical_size, UVec2::new(960, 720));
        }
    }
}
