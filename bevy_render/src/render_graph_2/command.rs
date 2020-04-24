use crate::{renderer_2::RenderContext, render_resource::RenderResource};
use std::{sync::{Arc, Mutex}, collections::VecDeque};

pub enum Command {
    CopyBufferToBuffer {
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    },
}

#[derive(Default, Clone)]
pub struct CommandQueue {
    // TODO: this shouldn't really need a mutex. its just needs to be shared on whatever thread its scheduled on
    queue: Arc<Mutex<VecDeque<Command>>>,
}

impl CommandQueue {
    fn push(&mut self, command: Command) {
        self.queue.lock().unwrap().push_front(command);
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        self.push(Command::CopyBufferToBuffer {
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        });
    }

    pub fn execute(&mut self, render_context: &mut dyn RenderContext) {
        for command in self.queue.lock().unwrap().drain(..) {
            match command {
                Command::CopyBufferToBuffer {
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                } => render_context.copy_buffer_to_buffer(
                    source_buffer,
                    source_offset,
                    destination_buffer,
                    destination_offset,
                    size,
                ),
            }
        }
    }
}