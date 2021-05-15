use std::sync::Arc;

use crate::{hand_tracking::HandPoseState, OpenXRStruct, XRState, XRSwapchain, XRViewTransform};

#[derive(Default)]
pub struct XRDevice {
    pub(crate) inner: Option<OpenXRStruct>, // FIXME remove option
    pub(crate) swapchain: Option<XRSwapchain>,
}

impl XRDevice {
    pub fn new(xr_struct: OpenXRStruct) -> Self {
        Self {
            inner: Some(xr_struct),
            swapchain: None,
        }
    }

    pub fn touch_update(&mut self) -> XRState {
        if self.swapchain.is_none() {
            return XRState::Paused; // FIXME or uninitialized?
        }

        self.swapchain
            .as_mut()
            .unwrap()
            .prepare_update(&mut self.inner.as_mut().unwrap().handles)
    }

    pub fn get_hand_positions(&mut self) -> Option<HandPoseState> {
        if self.swapchain.is_none() {
            return None;
        }

        self.swapchain
            .as_mut()
            .unwrap()
            .get_hand_positions(&mut self.inner.as_mut().unwrap().handles)
    }

    pub fn prepare_update(&mut self, device: &Arc<wgpu::Device>) -> XRState {
        if self.swapchain.is_none() {
            let xr_swapchain = XRSwapchain::new(device.clone(), self.inner.as_mut().unwrap());

            self.swapchain = Some(xr_swapchain);
        }

        self.swapchain
            .as_mut()
            .unwrap()
            .prepare_update(&mut self.inner.as_mut().unwrap().handles)
    }

    pub fn get_view_positions(&mut self) -> Option<Vec<XRViewTransform>> {
        if !self.inner.as_mut().unwrap().is_running() {
            return None;
        }

        let swapchain = match self.swapchain.as_mut() {
            None => return None,
            Some(sc) => sc,
        };

        Some(swapchain.get_view_positions(&mut self.inner.as_mut().unwrap().handles))
    }

    pub fn finalize_update(&mut self) {
        self.swapchain
            .as_mut()
            .unwrap()
            .finalize_update(&mut self.inner.as_mut().unwrap().handles);
    }

    pub fn get_swapchain_mut(&mut self) -> Option<&mut XRSwapchain> {
        Some(self.swapchain.as_mut()?)
    }
}

// FIXME FIXME FIXME ?!
unsafe impl Sync for XRDevice {}
unsafe impl Send for XRDevice {}
