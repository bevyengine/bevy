use bevy_platform_support::{
    prelude::Vec,
    sync::{
        atomic::{AtomicPtr, AtomicUsize, Ordering},
        Arc,
    },
};
use core::mem::MaybeUninit;

use bevy_utils::syncunsafecell::SyncUnsafeCell;

use super::Entity;

/// This is the item we store in the owned buffers.
/// It might not be init (if it's out of bounds).
/// It's in an unsafe cell (makes things simpler.)
type Slot = MaybeUninit<SyncUnsafeCell<Entity>>;

/// Each chunk stores a buffer of [`Slot`]s at a fixed capacity.
struct Chunk {
    /// Points to the first slot. If this is null, we need to allocate it.
    first: AtomicPtr<Slot>,
}

/// This is the shared data for the owned list.
/// It is the source of truth.
struct OwnedBuffer {
    /// Each chunk has a length the power of 2. The first has length 256 (2^8) and the last has length (2^31)
    chunks: [Chunk; 24],
    /// This is the total length of the buffer; the sum of each of the [`chunks`](Self::chunks)'s length.
    len: AtomicUsize,
    /// This points to the index in this buffer of the first [`Slot`] that is pending reuse.
    free_cursor: AtomicUsize,
}

/// This is the owned list.
/// It contains all entities owned by an allocator.
/// This includes empty archetype entities and entities pending reuse.
pub struct Owned {
    buffer: Arc<OwnedBuffer>,
}
