use std::ops::{Deref, DerefMut};

use crate::{
    render_resource::{SwapChainDescriptor, TextureViewId},
    renderer::RenderResources,
    RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_utils::HashMap;
use bevy_window::{RawWindowHandleWrapper, WindowId, Windows};

pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(0);
        render_app
            .add_system_to_stage(RenderStage::Extract, extract_windows.system())
            .add_system_to_stage(RenderStage::Prepare, prepare_windows.system());
    }
}

pub struct ExtractedWindow {
    pub id: WindowId,
    pub handle: RawWindowHandleWrapper,
    pub physical_width: u32,
    pub physical_height: u32,
    pub vsync: bool,
    pub swap_chain_texture: Option<TextureViewId>,
}

#[derive(Default)]
pub struct ExtractedWindows {
    pub windows: HashMap<WindowId, ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = HashMap<WindowId, ExtractedWindow>;

    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

fn extract_windows(mut commands: Commands, windows: Res<Windows>) {
    let mut extracted_windows = ExtractedWindows::default();
    for window in windows.iter() {
        extracted_windows.insert(
            window.id(),
            ExtractedWindow {
                id: window.id(),
                handle: window.raw_window_handle(),
                physical_width: window.physical_width(),
                physical_height: window.physical_height(),
                vsync: window.vsync(),
                swap_chain_texture: None,
            },
        );
    }

    commands.insert_resource(extracted_windows);
}

pub fn prepare_windows(
    mut windows: ResMut<ExtractedWindows>,
    render_resources: Res<RenderResources>,
) {
    for window in windows.windows.values_mut() {
        let swap_chain_descriptor = SwapChainDescriptor {
            window_id: window.id,
            format: crate::texture::TextureFormat::Bgra8UnormSrgb,
            width: window.physical_width,
            height: window.physical_height,
            vsync: window.vsync,
        };

        let swap_chain_texture = render_resources.next_swap_chain_texture(&swap_chain_descriptor);
        window.swap_chain_texture = Some(swap_chain_texture);
    }
}
