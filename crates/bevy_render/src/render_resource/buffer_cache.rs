use crate::{
    render_resource::{Buffer, BufferDescriptor},
    renderer::RenderDevice,
};
use bevy_ecs::{prelude::ResMut, system::Resource};
use bevy_utils::{Entry, HashMap};

struct CachedBufferMeta {
    buffer: Buffer,
    taken: bool,
    frames_since_last_use: usize,
}

pub struct CachedBuffer {
    pub buffer: Buffer,
}

#[derive(Resource, Default)]
pub struct BufferCache {
    buffers: HashMap<BufferDescriptor<'static>, Vec<CachedBufferMeta>>,
}

impl BufferCache {
    pub fn get(
        &mut self,
        render_device: &RenderDevice,
        descriptor: BufferDescriptor<'static>,
    ) -> CachedBuffer {
        match self.buffers.entry(descriptor) {
            Entry::Occupied(mut entry) => {
                for buffer in entry.get_mut().iter_mut() {
                    if !buffer.taken {
                        buffer.frames_since_last_use = 0;
                        buffer.taken = true;
                        return CachedBuffer {
                            buffer: buffer.buffer.clone(),
                        };
                    }
                }

                let buffer = render_device.create_buffer(&entry.key().clone());
                entry.get_mut().push(CachedBufferMeta {
                    buffer: buffer.clone(),
                    frames_since_last_use: 0,
                    taken: true,
                });
                CachedBuffer { buffer }
            }
            Entry::Vacant(entry) => {
                let buffer = render_device.create_buffer(entry.key());
                entry.insert(vec![CachedBufferMeta {
                    buffer: buffer.clone(),
                    taken: true,
                    frames_since_last_use: 0,
                }]);
                CachedBuffer { buffer }
            }
        }
    }

    pub fn update(&mut self) {
        for buffers in self.buffers.values_mut() {
            for buffer in buffers.iter_mut() {
                buffer.frames_since_last_use += 1;
                buffer.taken = false;
            }

            buffers.retain(|buffer| buffer.frames_since_last_use < 3);
        }
    }
}

pub fn update_buffer_cache_system(mut buffer_cache: ResMut<BufferCache>) {
    buffer_cache.update();
}
