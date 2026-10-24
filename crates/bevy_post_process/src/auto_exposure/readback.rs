use bevy_platform::sync::{Arc, Mutex, MutexGuard, PoisonError};
use bevy_render::{
    render_resource::{
        encase, Buffer, BufferDescriptor, BufferUsages, CommandEncoder, MapMode, ShaderType,
    },
    renderer::RenderDevice,
};

use super::pipeline::GpuAutoExposureState;

/// Staging buffers per view. A copy is made every frame while one is free; its map usually
/// completes one to three frames later.
const STAGING_BUFFERS: usize = 3;

/// Reads a view's adapted exposure back to the CPU without waiting on the GPU.
///
/// Each frame the auto exposure pass copies its state buffer into a free staging buffer, which is
/// mapped once the frame's commands are submitted. The newest completed copy is the read-back
/// exposure.
pub(super) struct StateReadback(Arc<Mutex<Readback>>);

struct Readback {
    free: Vec<Buffer>,
    copies: Copies,
}

/// Numbers copies and keeps the newest one made since the latest reset.
#[derive(Default)]
struct Copies {
    /// The number of copies made, which numbers the next one.
    made: u64,
    /// The first copy made since the latest reset.
    first_valid: u64,
    /// The newest valid copy and its adapted exposure in EV100.
    latest: Option<(u64, f32)>,
}

impl Copies {
    fn next(&mut self) -> u64 {
        self.made += 1;
        self.made - 1
    }

    fn reset(&mut self) {
        self.first_valid = self.made;
        self.latest = None;
    }

    /// Keeps `state` if its copy was made since the latest reset, holds an adapted exposure, and
    /// is newer than the one kept.
    fn complete(&mut self, copy: u64, state: Option<GpuAutoExposureState>) {
        if copy < self.first_valid || self.latest.is_some_and(|(newest, _)| newest > copy) {
            return;
        }
        if let Some(state) = state.filter(|state| state.initialized != 0) {
            self.latest = Some((copy, state.ev100));
        }
    }
}

fn lock(readback: &Mutex<Readback>) -> MutexGuard<'_, Readback> {
    readback.lock().unwrap_or_else(PoisonError::into_inner)
}

impl StateReadback {
    pub(super) fn new(device: &RenderDevice) -> Self {
        let free = (0..STAGING_BUFFERS)
            .map(|_| {
                device.create_buffer(&BufferDescriptor {
                    label: Some("auto_exposure_state_readback"),
                    size: GpuAutoExposureState::min_size().get(),
                    usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                })
            })
            .collect();
        Self(Arc::new(Mutex::new(Readback {
            free,
            copies: Copies::default(),
        })))
    }

    /// The adapted exposure, in EV100, from the newest copy made since the latest reset.
    pub(super) fn ev100(&self) -> Option<f32> {
        lock(&self.0).copies.latest.map(|(_, ev100)| ev100)
    }

    /// Forgets the read-back exposure and ignores the copies made so far.
    pub(super) fn reset(&self) {
        lock(&self.0).copies.reset();
    }

    /// Copies `state` into a free staging buffer, if one is free, and maps it once `encoder` is
    /// submitted.
    pub(super) fn copy(&self, encoder: &mut CommandEncoder, state: &Buffer) {
        let (staging, copy) = {
            let mut readback = lock(&self.0);
            let Some(staging) = readback.free.pop() else {
                return;
            };
            (staging, readback.copies.next())
        };
        encoder.copy_buffer_to_buffer(state, 0, &staging, 0, staging.size());
        let readback = self.0.clone();
        let mapped = staging.clone();
        encoder.map_buffer_on_submit(&staging, MapMode::Read, .., move |result| {
            let state = result.ok().and_then(|()| {
                let state = mapped
                    .slice(..)
                    .get_mapped_range()
                    .ok()
                    .and_then(|data| decode(&data));
                mapped.unmap();
                state
            });
            let mut readback = lock(&readback);
            readback.copies.complete(copy, state);
            readback.free.push(mapped);
        });
    }
}

fn decode(bytes: &[u8]) -> Option<GpuAutoExposureState> {
    encase::StorageBuffer::new(bytes).create().ok()
}

#[cfg(test)]
mod tests {
    use super::{decode, Copies, GpuAutoExposureState};

    fn state(ev100: f32) -> Option<GpuAutoExposureState> {
        Some(GpuAutoExposureState {
            ev100,
            initialized: 1,
        })
    }

    #[test]
    fn keeps_the_newest_copy() {
        let mut copies = Copies::default();
        let (a, b) = (copies.next(), copies.next());
        copies.complete(b, state(2.0));
        copies.complete(a, state(1.0));
        assert_eq!(copies.latest, Some((1, 2.0)));
    }

    #[test]
    fn ignores_copies_made_before_a_reset() {
        let mut copies = Copies::default();
        let before = copies.next();
        copies.complete(before, state(1.0));
        let in_flight = copies.next();
        copies.reset();
        assert_eq!(copies.latest, None);
        copies.complete(in_flight, state(2.0));
        assert_eq!(copies.latest, None);
        let after = copies.next();
        copies.complete(after, state(3.0));
        assert_eq!(copies.latest, Some((2, 3.0)));
    }

    #[test]
    fn ignores_failed_maps_and_unadapted_states() {
        let mut copies = Copies::default();
        let (a, b) = (copies.next(), copies.next());
        copies.complete(a, None);
        copies.complete(
            b,
            Some(GpuAutoExposureState {
                ev100: 0.0,
                initialized: 0,
            }),
        );
        assert_eq!(copies.latest, None);
    }

    #[test]
    fn decodes_the_state_layout() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&9.5f32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let state = decode(&bytes).unwrap();
        assert_eq!((state.ev100, state.initialized), (9.5, 1));
    }
}
