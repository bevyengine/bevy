use std::mem::{self, MaybeUninit};

use super::Command;
use crate::world::World;

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce the function pointer `func`.
    /// Returns the size of `T` in bytes.
    func: unsafe fn(value: *mut MaybeUninit<u8>, world: &mut World) -> usize,
}

/// A queue of [`Command`]s
//
// NOTE: [`CommandQueue`] is implemented via a `Vec<MaybeUninit<u8>>` instead of a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
#[derive(Default)]
pub struct CommandQueue {
    // This contiguously stores a set of alternating objects:
    // A `CommandMeta`, followed by a sequence of bytes with length specified by the metadata.
    // These bytes hold the data for a type-erased `Command`, and must be passed to
    // the corresponding `CommandMeta.func` to be used.
    bytes: Vec<MaybeUninit<u8>>,
}

// SAFETY: All commands [`Command`] implement [`Send`]
unsafe impl Send for CommandQueue {}

// SAFETY: `&CommandQueue` never gives access to the inner commands.
unsafe impl Sync for CommandQueue {}

impl CommandQueue {
    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<C>(&mut self, command: C)
    where
        C: Command,
    {
        /// SAFETY: This function is only every called when the `command` bytes is the associated
        /// [`Commands`] `T` type. Also this only reads the data via `read_unaligned` so unaligned
        /// accesses are safe.
        unsafe fn write_command_and_get_size<T: Command>(
            command: *mut MaybeUninit<u8>,
            world: &mut World,
        ) -> usize {
            let command = command.cast::<T>().read_unaligned();
            command.write(world);
            mem::size_of::<T>()
        }

        let meta = CommandMeta {
            func: write_command_and_get_size::<C>,
        };

        let old_len = self.bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        self.bytes
            .reserve(mem::size_of::<CommandMeta>() + mem::size_of::<C>());

        // Pointer to the bytes at the end of the buffer.
        // SAFETY: We know it is within bounds of the allocation, due to the call to `.reserve()`.
        let ptr = unsafe { self.bytes.as_mut_ptr().add(old_len) };

        // SAFETY: The end of the buffer has enough space for the metadata due to the `.reserve()` call,
        // so we can cast it to a pointer and perform an unaligned write in order to fill the buffer.
        // Since the buffer is of type `MaybeUninit<u8>`, any byte patterns are valid.
        unsafe {
            ptr.cast::<CommandMeta>().write_unaligned(meta);
        }

        if mem::size_of::<C>() > 0 {
            // SAFETY: There is enough space after the metadata to store the command,
            // due to the `.reserve()` call above.
            // We will write to the buffer via an unaligned pointer write.
            // Since the underlying buffer is of type `MaybeUninit<u8>`, any byte patterns are valid.
            unsafe {
                ptr.add(mem::size_of::<CommandMeta>())
                    .cast::<C>()
                    .write_unaligned(command);
            }
        }

        // Extend the length of the buffer to include the data we just wrote.
        // SAFETY: The new length is guaranteed to fit in the vector's capacity,
        // due to the call to `.reserve()` above.
        unsafe {
            self.bytes
                .set_len(mem::size_of::<CommandMeta>() + mem::size_of::<C>() + old_len);
        }
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // Cursor that will iterate over the entries in the buffer.
        // It will alternate between values of type `CommandMeta` and a values of unknown types.
        let mut cursor = self.bytes.as_mut_ptr();

        // The address of the end of the buffer.
        let end_addr = cursor as usize + self.bytes.len();

        // SAFETY: In the iteration below, `meta.func` will safely consume and drop each pushed command.
        // This operation is so that we can reuse the bytes `Vec<u8>`'s internal storage and prevent
        // unnecessary allocations.
        unsafe { self.bytes.set_len(0) };

        while (cursor as usize) < end_addr {
            // SAFETY: The bytes at `offset` are known to represent a value of type `CommandMeta`,
            // since the buffer alternates between storing `CommandMeta` and unknown bytes.
            // Its value will have been fully initialized during any calls to `push`.
            let meta = unsafe { cursor.cast::<CommandMeta>().read_unaligned() };
            // Advance to the bytes just after `meta`, which represent a type-erased command.
            // SAFETY: For most types of `Command`, the pointer immediately following the metadata
            // is guaranteed to be in bounds.
            // The pointer might be out of bounds if the command is zero-sized,
            // but it is okay to have a dangling pointer to a ZST.
            cursor = unsafe { cursor.add(mem::size_of::<CommandMeta>()) };
            // SAFETY: The type currently under the cursor must be the same type erased by `meta.func`.
            // We know that they are the same type, since they were stored next to each other by `.push()`.
            let size = unsafe { (meta.func)(cursor, world) };
            // Advance the cursor past the command.
            // SAFETY: At this point, it will either point to the next `CommandMeta`,
            // or the cursor will be out of bounds and the loop will end.
            cursor = unsafe { cursor.add(size) };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::{
        panic::AssertUnwindSafe,
        sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        },
    };

    struct DropCheck(Arc<AtomicU32>);

    impl DropCheck {
        fn new() -> (Self, Arc<AtomicU32>) {
            let drops = Arc::new(AtomicU32::new(0));
            (Self(drops.clone()), drops)
        }
    }

    impl Drop for DropCheck {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl Command for DropCheck {
        fn write(self, _: &mut World) {}
    }

    #[test]
    fn test_command_queue_inner_drop() {
        let mut queue = CommandQueue::default();

        let (dropcheck_a, drops_a) = DropCheck::new();
        let (dropcheck_b, drops_b) = DropCheck::new();

        queue.push(dropcheck_a);
        queue.push(dropcheck_b);

        assert_eq!(drops_a.load(Ordering::Relaxed), 0);
        assert_eq!(drops_b.load(Ordering::Relaxed), 0);

        let mut world = World::new();
        queue.apply(&mut world);

        assert_eq!(drops_a.load(Ordering::Relaxed), 1);
        assert_eq!(drops_b.load(Ordering::Relaxed), 1);
    }

    struct SpawnCommand;

    impl Command for SpawnCommand {
        fn write(self, world: &mut World) {
            world.spawn_empty();
        }
    }

    #[test]
    fn test_command_queue_inner() {
        let mut queue = CommandQueue::default();

        queue.push(SpawnCommand);
        queue.push(SpawnCommand);

        let mut world = World::new();
        queue.apply(&mut world);

        assert_eq!(world.entities().len(), 2);

        // The previous call to `apply` cleared the queue.
        // This call should do nothing.
        queue.apply(&mut world);
        assert_eq!(world.entities().len(), 2);
    }

    // This has an arbitrary value `String` stored to ensure
    // when then command gets pushed, the `bytes` vector gets
    // some data added to it.
    struct PanicCommand(String);
    impl Command for PanicCommand {
        fn write(self, _: &mut World) {
            panic!("command is panicking");
        }
    }

    #[test]
    fn test_command_queue_inner_panic_safe() {
        std::panic::set_hook(Box::new(|_| {}));

        let mut queue = CommandQueue::default();

        queue.push(PanicCommand("I panic!".to_owned()));
        queue.push(SpawnCommand);

        let mut world = World::new();

        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            queue.apply(&mut world);
        }));

        // even though the first command panicking.
        // the `bytes`/`metas` vectors were cleared.
        assert_eq!(queue.bytes.len(), 0);

        // Even though the first command panicked, it's still ok to push
        // more commands.
        queue.push(SpawnCommand);
        queue.push(SpawnCommand);
        queue.apply(&mut world);
        assert_eq!(world.entities().len(), 2);
    }

    // NOTE: `CommandQueue` is `Send` because `Command` is send.
    // If the `Command` trait gets reworked to be non-send, `CommandQueue`
    // should be reworked.
    // This test asserts that Command types are send.
    fn assert_is_send_impl(_: impl Send) {}
    fn assert_is_send(command: impl Command) {
        assert_is_send_impl(command);
    }

    #[test]
    fn test_command_is_send() {
        assert_is_send(SpawnCommand);
    }

    struct CommandWithPadding(u8, u16);
    impl Command for CommandWithPadding {
        fn write(self, _: &mut World) {}
    }

    #[cfg(miri)]
    #[test]
    fn test_uninit_bytes() {
        let mut queue = CommandQueue::default();
        queue.push(CommandWithPadding(0, 0));
        let _ = format!("{:?}", queue.bytes);
    }
}
