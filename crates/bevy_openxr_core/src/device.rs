use std::sync::Arc;

use crate::{
    event::{XREvent, XRViewCreated},
    hand_tracking::HandPoseState,
    OpenXRStruct, XRState, XRSwapchain, XRViewTransform,
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
            let swapchain = XRSwapchain::new(device.clone(), &mut self.inner);
            let resolution = swapchain.get_resolution();
            self.events_to_send
                .push(XREvent::ViewCreated(XRViewCreated {
                    width: resolution.0,
                    height: resolution.1,
                }));
            self.swapchain = Some(swapchain);
        }

        // call swapchain update
        self.swapchain
            .as_mut()
            .unwrap()
            .prepare_update(&mut self.inner.handles)
    }

    pub fn get_view_positions(&mut self) -> Option<Vec<XRViewTransform>> {
        if !self.inner.is_running() {
            return None;
        }

        let swapchain = match self.swapchain.as_mut() {
            None => return None,
            Some(sc) => sc,
        };

        Some(swapchain.get_view_positions(&mut self.inner.handles))
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
