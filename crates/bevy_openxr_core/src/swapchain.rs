use bevy_math::{Quat, Vec3};
use bevy_transform::components::Transform;
use bevy_utils::tracing::warn;
use openxr::{HandJointLocations, Time, View};
use std::{fmt::Debug, num::NonZeroU32, sync::Arc};
use wgpu::OpenXRHandles;

use crate::{
    hand_tracking::{HandPoseState, HandTrackers},
    OpenXRStruct, XRState,
};

pub struct XRSwapchain {
    /// OpenXR internal swapchain handle
    sc_handle: openxr::Swapchain<openxr::Vulkan>,

    /// Swapchain Framebuffers. `XRSwapchainNode` will take ownership of the color buffer
    buffers: Vec<Framebuffer>,

    /// Swapchain resolution
    resolution: wgpu::Extent3d,

    /// Swapchain view configuration type
    view_configuration_type: openxr::ViewConfigurationType,

    /// Desired environment blend mode
    environment_blend_mode: openxr::EnvironmentBlendMode,

    /// Rendering and prediction information for the next frame
    next_frame_state: Option<openxr::FrameState>,

    /// TODO: move this away, doesn't belong here
    hand_trackers: Option<HandTrackers>,
}

const VIEW_COUNT: u32 = 2; // FIXME get from settings
const COLOR_FORMAT: ash::vk::Format = ash::vk::Format::R8G8B8A8_UNORM; // FIXME change!!

impl XRSwapchain {
    pub fn new(device: Arc<wgpu::Device>, openxr_struct: &mut OpenXRStruct) -> Self {
        let views = openxr_struct
            .instance
            .enumerate_view_configuration_views(
                openxr_struct.handles.system,
                openxr_struct.options.view_type,
            )
            .unwrap();

        assert_eq!(views.len(), VIEW_COUNT as usize);
        assert_eq!(views[0], views[1]);

        println!("Enumerated OpenXR views: {:#?}", views);

        let resolution = wgpu::Extent3d {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
            depth_or_array_layers: 1,
        };

        let format = wgpu::TextureFormat::Rgba8Unorm;

        let handle = openxr_struct
            .handles
            .session
            .create_swapchain(&openxr::SwapchainCreateInfo {
                create_flags: openxr::SwapchainCreateFlags::EMPTY,
                usage_flags: openxr::SwapchainUsageFlags::COLOR_ATTACHMENT
                 ,/*   | openxr::SwapchainUsageFlags::DEPTH_STENCIL_ATTACHMENT, // FIXME depth?
                | openxr::SwapchainUsageFlags::SAMPLED, | openxr::SwapchainUsageFlags::TRANSFER_SRC
                | openxr::SwapchainUsageFlags::TRANSFER_DST,*/
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
                    texture_view: Some(color),
                }
            })
            .collect();

        let hand_trackers = if openxr_struct.options.hand_trackers {
            // FIXME check feature
            Some(HandTrackers::new(&openxr_struct.handles.session).unwrap())
        } else {
            None
        };

        XRSwapchain {
            sc_handle: handle,
            buffers,
            resolution,
            view_configuration_type: openxr_struct.options.view_type,
            environment_blend_mode,
            next_frame_state: None,
            hand_trackers,
        }
    }

    /// Return the next swapchain image index to render into
    /// FIXME: currently waits for compositor to release image for rendering, this might cause delays in bevy system
    ///        (e.g. should wait somewhere else - but how to use handle there)
    pub fn get_next_swapchain_image_index(&mut self) -> usize {
        let image_index = self.sc_handle.acquire_image().unwrap();
        self.sc_handle
            .wait_image(openxr::Duration::INFINITE)
            .unwrap();
        image_index as usize
    }

    /// Prepares the device for rendering. Called before each frame is rendered
    pub fn prepare_update(&mut self, handles: &mut OpenXRHandles) -> XRState {
        // Check that previous frame was rendered
        if let Some(_) = self.next_frame_state {
            warn!("Called prepare_update() even though it was called already");
            return XRState::Running; // <-- FIXME might change state, should keep it in memory somewhere
        }

        let frame_state = match handles.frame_waiter.wait() {
            Ok(fs) => fs,
            Err(_) => {
                // FIXME handle this better
                return XRState::Paused;
            }
        };

        // 'Indicate that graphics device work is beginning'
        handles.frame_stream.begin().unwrap();

        if !frame_state.should_render {
            // if false, "the application should avoid heavy GPU work where possible" (openxr spec)
            handles
                .frame_stream
                .end(
                    frame_state.predicted_display_time,
                    self.environment_blend_mode,
                    &[],
                )
                .unwrap();

            return XRState::Paused;
        }

        // All ok for rendering
        self.next_frame_state = Some(frame_state);
        return XRState::Running;
    }

    /// TODO: move this away, doesn't belong here
    pub fn get_hand_positions(&mut self, handles: &mut OpenXRHandles) -> Option<HandPoseState> {
        let frame_state = match self.next_frame_state {
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

    pub fn get_view_positions(&mut self, handles: &mut OpenXRHandles) -> Option<Vec<Transform>> {
        if let None = self.next_frame_state {
            self.prepare_update(handles);
        }

        if let None = self.next_frame_state {
            return None;
        }

        let frame_state = self.next_frame_state.as_ref().unwrap();

        // FIXME views acquisition should probably occur somewhere else - timing problem?
        let (_, views) = handles
            .session
            .locate_views(
                self.view_configuration_type,
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
                let mut transform = Transform::from_translation(Vec3::new(pos.x, pos.y, pos.z));
                transform.rotation = Quat::from_xyzw(ori.x, ori.y, ori.z, ori.w);
                transform
            })
            .collect();

        //println!("TRANSFORMS: {:#?}", transforms);
        Some(transforms)
    }

    /// Finalizes the swapchain update - will tell openxr that GPU has rendered to textures
    pub fn finalize_update(&mut self, handles: &mut OpenXRHandles) {
        // "Release the oldest acquired image"
        self.sc_handle.release_image().unwrap();

        // Take the next frame state
        let next_frame_state = self.next_frame_state.take().unwrap();

        // FIXME views acquisition should probably occur somewhere else - timing problem?
        // FIXME is there a problem now, if the rendering uses different camera positions than what's used at openxr?
        // "When rendering, this should be called as late as possible before the GPU accesses it to"
        let (_, views) = handles
            .session
            .locate_views(
                self.view_configuration_type,
                next_frame_state.predicted_display_time,
                &handles.space,
            )
            .unwrap();

        // Tell OpenXR what to present for this frame
        // Because we're using GL_EXT_multiview, same rect for both eyes
        let rect = openxr::Rect2Di {
            offset: openxr::Offset2Di { x: 0, y: 0 },
            extent: openxr::Extent2Di {
                width: self.resolution.width as _,
                height: self.resolution.height as _,
            },
        };

        // Construct views
        // TODO: for performance (no-vec allocations), use `SmallVec`?
        let views = views
            .iter()
            .enumerate()
            .map(|(idx, view)| {
                openxr::CompositionLayerProjectionView::new()
                    .pose(view.pose)
                    .fov(view.fov)
                    .sub_image(
                        openxr::SwapchainSubImage::new()
                            .swapchain(&self.sc_handle)
                            .image_array_index(idx as u32)
                            .image_rect(rect),
                    )
            })
            .collect::<Vec<_>>();

        handles
            .frame_stream
            .end(
                next_frame_state.predicted_display_time,
                self.environment_blend_mode,
                &[&openxr::CompositionLayerProjection::new()
                    .space(&handles.space)
                    .views(&views)],
            )
            .unwrap();
    }

    /// Should be called only once by `XRSwapchainNode`
    pub fn take_texture_views(&mut self) -> Vec<wgpu::TextureView> {
        self.buffers
            .iter_mut()
            .map(|buf| buf.texture_view.take().unwrap())
            .collect()
    }

    pub fn get_resolution(&self) -> (u32, u32) {
        (self.resolution.width, self.resolution.height)
    }

    pub fn get_views(&self, handles: &mut OpenXRHandles) -> Vec<View> {
        let (_, views) = handles
            .session
            .locate_views(
                self.view_configuration_type,
                Time::from_nanos(1), // FIXME time must be non-zero, is this okay?
                &handles.space,
            )
            .unwrap();

        views
    }
}

impl Debug for XRSwapchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XRSwapchain[]")
    }
}

/// Per view framebuffer, that will contain an underlying texture and a texture view (taken away by bevy render graph)
/// where the contents should be rendered
struct Framebuffer {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    texture_view: Option<wgpu::TextureView>,
}
