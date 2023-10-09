use std::sync::{mpsc::{Receiver, channel}, Arc, Mutex};

use bevy_ecs::prelude::*;
use rodio::cpal::{self, InputCallbackInfo, traits::{DeviceTrait, HostTrait}};

/// Used internally to retrieve audio on the current "audio input device"
#[derive(Resource)]
pub struct AudioInput {
    stream: Option<Arc<Mutex<Receiver<Vec<f32>>>>>
}

impl AudioInput {
    pub fn try_default() -> Option<Self> {
        let host = cpal::default_host();
        let input = host.default_input_device()?;
        let config = input.default_input_config().ok()?;

        let (tx, rx) = channel();

        let stream = input.build_input_stream(
            &config.config(),
            move |data: &[f32], _: &InputCallbackInfo| {
                if let Err(error) = tx.send(data.into_iter().cloned().collect()) {
                    todo!("handle sending error")
                };
            },
            move |err| {
                todo!("handle reading error")
            },
            None
        ).ok()?;

        // We leak `Stream` to prevent the audio inputs from stopping.
        std::mem::forget(stream);

        let rx = Arc::new(Mutex::new(rx));

        Some(Self { stream: Some(rx) })
    }
}

impl Default for AudioInput {
    fn default() -> Self {
        Self::try_default().unwrap_or(Self { stream: None })
    }
}

