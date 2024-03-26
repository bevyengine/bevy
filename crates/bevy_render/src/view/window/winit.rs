//! Everything should be kept private since it's unlikely anyone needs to use this directly.
//! Instead, abstractions should be used like [`ExtractedWindows`].

use std::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_ecs::entity::{Entity, EntityHashMap};
use bevy_ecs::system::{NonSend, NonSendMut, Resource};
use bevy_winit::winit;
use crate::{Extract, ExtractSchedule, RenderApp};


/// This [`Plugin`] extracts [`WinitWindows`] into render world.
/// This is needed to avoid crashes *when using pipelined rendering* since the winit window can be
/// modified or removed from app world.
pub struct WinitWindowRenderPlugin;

impl Plugin for WinitWindowRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_non_send_resource::<ExtractedWinitWindows>()
                .add_systems(ExtractSchedule, extract_winit_windows);
        }
    }
}

#[allow(dead_code)]
struct ExtractedWinitWindow{
    entity: Entity,
    window: Arc<winit::window::Window>,
    window_id: winit::window::WindowId,
}

#[derive(Resource, Default)]
struct ExtractedWinitWindows {
    windows: EntityHashMap<ExtractedWinitWindow>,
}

fn extract_winit_windows(
    mut extracted_windows: NonSendMut<ExtractedWinitWindows>,
    windows: Extract<NonSend<bevy_winit::WinitWindows>>,
){
    extracted_windows.windows.clear();
    for (&window_id, window) in windows.windows.iter(){
        let entity = windows.winit_to_entity.get(&window_id).expect("Window has no entity mapped. This should be impossible.");
        extracted_windows.windows.insert(*entity, ExtractedWinitWindow {
            window_id,
            window: window.clone(),
            entity: entity.clone(),
        });
    }
}