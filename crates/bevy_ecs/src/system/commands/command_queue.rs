use std::{mem::MaybeUninit, ptr::NonNull};

use bevy_ptr::{OwningPtr, Unaligned};

use super::Command;
use crate::world::World;

struct CommandMeta {
    /// Offset from the start of `CommandQueue.bytes` at which the corresponding command is stored.
    offset: usize,
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    apply_command: unsafe fn(value: OwningPtr<Unaligned>, world: &mut World),
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
    /// Densely stores the data for all commands in the queue.
    bytes: Vec<MaybeUninit<u8>>,
    /// Metadata for each command stored in the queue.
    /// SAFETY: Each entry must have a corresponding value stored in `bytes`,
    /// stored at offset `CommandMeta.offset` and with an underlying type matching `CommandMeta.apply_command`.
    metas: Vec<CommandMeta>,
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
        let old_len = self.bytes.len();

        // SAFETY: After adding the metadata, we correctly write the corresponding `command`
        // of type `C` into `self.bytes`. Zero-sized commands do not get written into the buffer,
        // so we'll just use a dangling pointer, which is valid for zero-sized types.
        self.metas.push(CommandMeta {
            offset: old_len,
            apply_command: |command, world| {
                // SAFETY: According to the invariants of `CommandMeta.apply_command`,
                // `command` must point to a value of type `C`.
                let command: C = unsafe { command.read_unaligned() };
                command.write(world);
            },
        });

        let size = std::mem::size_of::<C>();
        if size > 0 {
            // Ensure that the buffer has enough space at the end to fit a value of type `C`.
            // Since `C` is non-zero sized, this also guarantees that the buffer is non-null.
            self.bytes.reserve(size);

            // SAFETY: The buffer must be at least as long as `old_len`, so this operation
            // will not overflow the pointer's original allocation.
            let ptr: *mut C = unsafe { self.bytes.as_mut_ptr().add(old_len).cast() };

            // Transfer ownership of the command into the buffer.
            // SAFETY: `ptr` must be non-null, since it is within a non-null buffer.
            // The call to `reserve()` ensures that the buffer has enough space to fit a value of type `C`,
            // and it is valid to write any bit pattern since the underlying buffer is of type `MaybeUninit<u8>`.
            unsafe { ptr.write_unaligned(command) };

            // Grow the vector to include the command we just wrote.
            // SAFETY: Due to the call to `.reserve(size)` above,
            // this is guaranteed to fit in the vector's capacity.
            unsafe { self.bytes.set_len(old_len + size) };
        } else {
            // Instead of writing zero-sized types into the buffer, we'll just use a dangling pointer.
            // We must forget the command so it doesn't get double-dropped when the queue gets applied.
            std::mem::forget(command);
        }
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // Reset the buffer, so it can be reused after this function ends.
        // In the loop below, ownership of each command will be transferred into user code.
        // SAFETY: `set_len(0)` is always valid.
        unsafe { self.bytes.set_len(0) };

        for meta in self.metas.drain(..) {
            // SAFETY: `CommandQueue` guarantees that each metadata must have a corresponding value stored in `self.bytes`,
            // so this addition will not overflow its original allocation.
            let cmd = unsafe { self.bytes.as_mut_ptr().add(meta.offset) };
            // SAFETY: It is safe to transfer ownership out of `self.bytes`, since the call to `set_len(0)` above
            // gaurantees that nothing stored in the buffer will get observed after this function ends.
            // `cmd` points to a valid address of a stored command, so it must be non-null.
            let cmd = unsafe { OwningPtr::new(NonNull::new_unchecked(cmd.cast())) };
            // SAFETY: The underlying type of `cmd` matches the type expected by `meta.apply_command`.
            unsafe {
                (meta.apply_command)(cmd, world);
            }
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
        assert_eq!(queue.metas.len(), 0);

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
