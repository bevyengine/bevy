use crate::WinitWindows;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use wasm_bindgen::JsCast;
use winit::dpi::LogicalSize;

pub(crate) struct CanvasParentResizePlugin;

impl Plugin for CanvasParentResizePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CanvasParentResizeEventChannel>()
            .add_systems(Update, canvas_parent_resize_event_handler);
    }
}

struct ResizeEvent {
    size: LogicalSize<f32>,
    window: Entity,
}

#[derive(Resource)]
pub(crate) struct CanvasParentResizeEventChannel {
    sender: Sender<ResizeEvent>,
    receiver: Receiver<ResizeEvent>,
}

fn canvas_parent_resize_event_handler(
    winit_windows: NonSend<WinitWindows>,
    resize_events: Res<CanvasParentResizeEventChannel>,
) {
    for event in resize_events.receiver.try_iter() {
        if let Some(window) = winit_windows.get_window(event.window) {
            window.set_inner_size(event.size);
        }
    }
}

fn get_size(selector: &str) -> Option<LogicalSize<f32>> {
    let win = web_sys::window().unwrap();
    let doc = win.document().unwrap();
    let element = doc.query_selector(selector).ok()??;
    let parent_element = element.parent_element()?;
    let rect = parent_element.get_bounding_client_rect();
    return Some(winit::dpi::LogicalSize::new(
        rect.width() as f32,
        rect.height() as f32,
    ));
}

pub(crate) const WINIT_CANVAS_SELECTOR: &str = "canvas[data-raw-handle]";

impl Default for CanvasParentResizeEventChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        return Self { sender, receiver };
    }
}

impl CanvasParentResizeEventChannel {
    pub(crate) fn listen_to_selector(&self, window: Entity, selector: &str) {
        let sender = self.sender.clone();
        let owned_selector = selector.to_string();
        let resize = move || {
            if let Some(size) = get_size(&owned_selector) {
                sender.send(ResizeEvent { size, window }).unwrap();
            }
        };

        // ensure resize happens on startup
        resize();

        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_: web_sys::Event| {
            resize();
        }) as Box<dyn FnMut(_)>);
        let window = web_sys::window().unwrap();

        window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
