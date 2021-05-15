use bevy_math::{Quat, Vec3};
use openxr::HandJointLocations;
use std::num::NonZeroU32;
use wgpu::OpenXRHandles;

use crate::{OpenXROptions, XRState, XRViewTransform};

pub struct XRSwapchain {
    handle: openxr::Swapchain<openxr::Vulkan>,
    buffers: Vec<Framebuffer>,
    resolution: wgpu::Extent3d,
    options: OpenXROptions,
    environment_blend_mode: openxr::EnvironmentBlendMode,
    frame_state: Option<openxr::FrameState>,
    hand_trackers: Option<HandTrackers>,
}

impl XRSwapchain {
    pub fn new(
        device: std::sync::Arc<wgpu::Device>,
        openxr_struct: &mut crate::OpenXRStruct,
    ) -> Self {
        const VIEW_COUNT: u32 = 2; // FIXME get from settings

        let views = openxr_struct
            .instance
            .enumerate_view_configuration_views(
                openxr_struct.handles.system,
                openxr_struct.options.view_type,
            )
            .unwrap();

        assert_eq!(views.len(), VIEW_COUNT as usize);
        assert_eq!(views[0], views[1]);

        println!("VIEWS: {:#?}", views);

        let resolution = wgpu::Extent3d {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
            depth_or_array_layers: 1,
        };

        const COLOR_FORMAT: ash::vk::Format = ash::vk::Format::R8G8B8A8_UNORM; // FIXME change!!
        let format = wgpu::TextureFormat::Rgba8Unorm;

        let handle = openxr_struct
            .handles
            .session
            .create_swapchain(&openxr::SwapchainCreateInfo {
                create_flags: openxr::SwapchainCreateFlags::EMPTY,
                usage_flags: openxr::SwapchainUsageFlags::COLOR_ATTACHMENT | openxr::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT // FIXME depth?
                | openxr::SwapchainUsageFlags::SAMPLED | openxr::SwapchainUsageFlags::TRANSFER_SRC
                | openxr::SwapchainUsageFlags::TRANSFER_DST,
                format: COLOR_FORMAT.as_raw() as _,
                sample_count: 1,
                width: resolution.width,
                height: resolution.height,
                face_count: 1,
                array_size: VIEW_COUNT,
                mip_count: 1,
            })
            .unwrap();

        let environment_blend_mode = openxr_struct
            .instance
            .enumerate_environment_blend_modes(
                openxr_struct.handles.system,
                openxr_struct.options.view_type,
            )
            .unwrap()[0];

        let images = handle.enumerate_images().unwrap();

        let buffers = images
            .into_iter()
            .map(|color_image| {
                // FIXME keep in sync with above usage_flags
                let texture = device.create_openxr_texture_from_raw_image(
                    &wgpu::TextureDescriptor {
                        size: wgpu::Extent3d {
                            width: resolution.width,
                            height: resolution.height,
                            depth_or_array_layers: 2,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsage::RENDER_ATTACHMENT
                            | wgpu::TextureUsage::STORAGE
                            | wgpu::TextureUsage::COPY_SRC
                            | wgpu::TextureUsage::COPY_DST,
                        label: None,
                    },
                    color_image,
                );

                let color = texture.create_view(&wgpu::TextureViewDescriptor {
                    label: None,
                    format: Some(format),
                    dimension: Some(wgpu::TextureViewDimension::D2Array),
                    aspect: wgpu::TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: NonZeroU32::new(1),
                    base_array_layer: 0,
                    array_layer_count: NonZeroU32::new(2),
                });

                Framebuffer {
                    texture,
                    color: Some(color),
                }
            })
            .collect();

        let options = openxr_struct.options.clone();

        let hand_trackers = if options.hand_trackers {
            // FIXME check feature
            Some(HandTrackers::new(&openxr_struct.handles.session).unwrap())
        } else {
            None
        };

        XRSwapchain {
            handle,
            buffers,
            resolution,
            options,
            environment_blend_mode,
            frame_state: None,
            hand_trackers,
        }
    }

    pub fn get_next_swapchain_image_index(&mut self) -> usize {
        let image_index = self.handle.acquire_image().unwrap();
        self.handle.wait_image(openxr::Duration::INFINITE).unwrap();
        image_index as usize
    }

    pub fn prepare_update(&mut self, handles: &mut OpenXRHandles) -> XRState {
        if let Some(_) = self.frame_state {
            return XRState::Running;
        }

        let xr_frame_state = match handles.frame_waiter.wait() {
            Ok(fs) => fs,
            Err(_) => {
                // FIXME handle this better
                return XRState::Paused;
            }
        };

        handles.frame_stream.begin().unwrap();

        if !xr_frame_state.should_render {
            handles
                .frame_stream
                .end(
                    xr_frame_state.predicted_display_time,
                    self.environment_blend_mode,
                    &[],
                )
                .unwrap();

            return XRState::Paused;
        }

        self.frame_state = Some(xr_frame_state);

        return XRState::Running;
    }

    pub fn get_hand_positions(&mut self, handles: &mut OpenXRHandles) -> Option<HandPoseState> {
        let frame_state = match self.frame_state {
            Some(fs) => fs,
            None => return None,
        };

        let ht = match &self.hand_trackers {
            Some(ht) => ht,
            None => return None,
        };

        let hand_l = handles
            .space
            .locate_hand_joints(&ht.tracker_l, frame_state.predicted_display_time)
            .unwrap();
        let hand_r = handles
            .space
            .locate_hand_joints(&ht.tracker_r, frame_state.predicted_display_time)
            .unwrap();

        let hand_pose_state = HandPoseState {
            left: hand_l,
            right: hand_r,
        };

        Some(hand_pose_state)
    }

    pub fn get_view_positions(&mut self, handles: &mut OpenXRHandles) -> Vec<XRViewTransform> {
        if let None = self.frame_state {
            self.prepare_update(handles);
        }

        let frame_state = self.frame_state.as_ref().unwrap();

        // FIXME views acquisition should probably occur somewhere else - timing problem?
        let (_, views) = handles
            .session
            .locate_views(
                self.options.view_type,
                frame_state.predicted_display_time,
                &handles.space,
            )
            .unwrap();

        //println!("VIEWS: {:#?}", views);

        let transforms = views
            .iter()
            .map(|view| {
                let pos = &view.pose.position;
                let ori = &view.pose.orientation;
                XRViewTransform::new(
                    Vec3::new(pos.x, pos.y, pos.z),
                    Quat::from_xyzw(ori.x, ori.y, ori.z, ori.w),
                )
            })
            .collect();

        //println!("TRANSFORMS: {:#?}", transforms);
        transforms
    }

    pub fn finalize_update(&mut self, handles: &mut OpenXRHandles) {
        self.handle.release_image().unwrap();
        let frame_state = self.frame_state.take().unwrap();

        // FIXME views acquisition should probably occur somewhere else - timing problem?
        let (_, views) = handles
            .session
            .locate_views(
                self.options.view_type,
                frame_state.predicted_display_time,
                &handles.space,
            )
            .unwrap();

        // Tell OpenXR what to present for this frame
        let rect = openxr::Rect2Di {
            offset: openxr::Offset2Di { x: 0, y: 0 },
            extent: openxr::Extent2Di {
                width: self.resolution.width as _,
                height: self.resolution.height as _,
            },
        };

        handles
            .frame_stream
            .end(
                frame_state.predicted_display_time,
                self.environment_blend_mode,
                &[&openxr::CompositionLayerProjection::new()
                    .space(&handles.space)
                    .views(&[
                        openxr::CompositionLayerProjectionView::new()
                            .pose(views[0].pose)
                            .fov(views[0].fov)
                            .sub_image(
                                openxr::SwapchainSubImage::new()
                                    .swapchain(&self.handle)
                                    .image_array_index(0)
                                    .image_rect(rect),
                            ),
                        openxr::CompositionLayerProjectionView::new()
                            .pose(views[1].pose)
                            .fov(views[1].fov)
                            .sub_image(
                                openxr::SwapchainSubImage::new()
                                    .swapchain(&self.handle)
                                    .image_array_index(1)
                                    .image_rect(rect),
                            ),
                    ])],
            )
            .unwrap();
    }

    pub fn take_color_textures(&mut self) -> Vec<wgpu::TextureView> {
        self.buffers
            .iter_mut()
            .map(|buf| buf.color.take().unwrap())
            .collect()
    }
}

impl std::fmt::Debug for XRSwapchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XRSwapchain[]")
    }
}

struct Framebuffer {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    color: Option<wgpu::TextureView>,
}

struct HandTrackers {
    tracker_l: openxr::HandTracker,
    tracker_r: openxr::HandTracker,
}

impl HandTrackers {
    pub fn new(session: &openxr::Session<openxr::Vulkan>) -> Result<Self, crate::Error> {
        let ht = HandTrackers {
            tracker_l: session.create_hand_tracker(openxr::HandEXT::LEFT)?,
            tracker_r: session.create_hand_tracker(openxr::HandEXT::RIGHT)?,
        };

        Ok(ht)
    }
}

#[derive(Default)]
pub struct HandPoseState {
    pub left: Option<HandJointLocations>,
    pub right: Option<HandJointLocations>,
}

impl std::fmt::Debug for HandPoseState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(left: {}, right: {})",
            self.left.is_some(),
            self.right.is_some()
        )
    }
}
