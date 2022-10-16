use crate::WinitWindows;
use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use wasm_bindgen::JsCast;
use web_sys::HtmlCanvasElement;
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

impl Default for CanvasParentResizeEventChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

fn get_size_element(element: &HtmlCanvasElement) -> Option<LogicalSize<f32>> {
    let parent_element = element.parent_element()?;
    let rect = parent_element.get_bounding_client_rect();
    Some(winit::dpi::LogicalSize::new(
        rect.width() as f32,
        rect.height() as f32,
    ))
}

impl CanvasParentResizeEventChannel {
    /// Listen to resize events on the element
    ///
    /// ## Panic
    ///
    /// Do not call from a web-worker!
    /// This method uses global `window` object to attach events to,
    /// which doesn't exist inside worker context.
    pub(crate) fn listen_to_element(&self, window: Entity, element: HtmlCanvasElement) {
        let sender = self.sender.clone();
        let resize = move || {
            if let Some(size) = get_size_element(&element) {
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
