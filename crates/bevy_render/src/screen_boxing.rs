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
