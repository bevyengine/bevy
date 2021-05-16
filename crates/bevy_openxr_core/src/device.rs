use std::sync::Arc;

use bevy_transform::components::Transform;
use openxr::Time;

use crate::{
    event::{XREvent, XRViewSurfaceCreated, XRViewsCreated},
    hand_tracking::HandPoseState,
    OpenXRStruct, XRState, XRSwapchain,
};

pub struct XRDevice {
    pub(crate) inner: OpenXRStruct,

    /// Swapchain. Must be `Option` because initializing swapchain requires access to `wgpu::Device`
    /// which is not available here - but rather at `bevy_wgpu`
    pub(crate) swapchain: Option<XRSwapchain>,

    /// Event collection to convert into bevy events
    events_to_send: Vec<XREvent>,
}

impl XRDevice {
    pub fn new(xr_struct: OpenXRStruct) -> Self {
        Self {
            inner: xr_struct,
            swapchain: None,
            events_to_send: Vec::new(),
        }
    }

    pub fn touch_update(&mut self) -> XRState {
        if self.swapchain.is_none() {
            return XRState::Paused; // FIXME or uninitialized?
        }

        self.swapchain
            .as_mut()
            .unwrap()
            .prepare_update(&mut self.inner.handles)
    }

    pub fn get_hand_positions(&mut self) -> Option<HandPoseState> {
        if self.swapchain.is_none() {
            return None;
        }

        self.swapchain
            .as_mut()
            .unwrap()
            .get_hand_positions(&mut self.inner.handles)
    }

    pub fn prepare_update(&mut self, device: &Arc<wgpu::Device>) -> XRState {
        // construct swapchain at first call
        if self.swapchain.is_none() {
            let mut swapchain = XRSwapchain::new(device.clone(), &mut self.inner);

            swapchain.prepare_update(&mut self.inner.handles);

            let views = swapchain
                .get_views(&mut self.inner.handles)
                .iter()
                .map(|view| View {
                    fov: XrFovf {
                        angle_left: view.fov.angle_left,
                        angle_right: view.fov.angle_right,
                        angle_down: view.fov.angle_down,
                        angle_up: view.fov.angle_up,
                    },
                })
                .collect::<Vec<View>>();

            let resolution = swapchain.get_resolution();
            println!(
                "Swapchain configured, resolution {:?}, views: {:#?}",
                resolution, views
            );

            self.events_to_send
                .push(XREvent::ViewSurfaceCreated(XRViewSurfaceCreated {
                    width: resolution.0,
                    height: resolution.1,
                }));

            self.events_to_send
                .push(XREvent::ViewsCreated(XRViewsCreated {
                    views: views.clone(),
                }));

            self.swapchain = Some(swapchain);

            // hack to prevent render graph panic when output has not been sent
            // what will happen after this: event will be sent about xr view, XRWindowTextureNode will configure itself at next frame
            // and after that all will be okay
            // this doesn't actually work on all cases... have to investigate
            return XRState::SkipFrame;
        }

        // call swapchain update
        self.swapchain
            .as_mut()
            .unwrap()
            .prepare_update(&mut self.inner.handles)
    }

    pub fn get_view_positions(&mut self) -> Option<Vec<Transform>> {
        if !self.inner.is_running() {
            return None;
        }

        let swapchain = match self.swapchain.as_mut() {
            None => return None,
            Some(sc) => sc,
        };

        swapchain.get_view_positions(&mut self.inner.handles)
    }

    pub fn finalize_update(&mut self) {
        self.swapchain
            .as_mut()
            .unwrap()
            .finalize_update(&mut self.inner.handles);
    }

    pub fn get_swapchain_mut(&mut self) -> Option<&mut XRSwapchain> {
        Some(self.swapchain.as_mut()?)
    }

    pub(crate) fn drain_events(&mut self) -> Vec<XREvent> {
        self.events_to_send.drain(..).collect()
    }
}

// FIXME FIXME FIXME ?!
unsafe impl Sync for XRDevice {}
unsafe impl Send for XRDevice {}

#[derive(Debug, Clone)]
pub struct View {
    pub fov: XrFovf,
}

#[derive(Debug, Clone)]
pub struct XrFovf {
    pub angle_left: f32,
    pub angle_right: f32,
    pub angle_down: f32,
    pub angle_up: f32,
}
