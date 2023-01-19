use std::mem::MaybeUninit;

use bevy_ptr::{OwningPtr, Unaligned};

use super::Command;
use crate::world::World;

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    ///
    /// Returns the size of `T` in bytes.
    apply_command_and_get_size: unsafe fn(value: OwningPtr<Unaligned>, world: &mut World) -> usize,
}

/// Densely and efficiently stores a queue of heterogenous types implementing [`Command`].
//
// NOTE: [`CommandQueue`] is implemented via a `Vec<MaybeUninit<u8>>` instead of a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
#[derive(Default)]
pub struct CommandQueue {
    // This buffer densely stores all queued commands.
    //
    // For each command, one `CommandMeta` is stored, followed by zero or more bytes
    // to store the command itself. To interpret these bytes, a pointer must
    // be passed to the corresponding `CommandMeta.apply_command_and_get_size` fn pointer.
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
        let meta = CommandMeta {
            apply_command_and_get_size: |ptr, world| {
                // SAFETY: The safety invariants of the field `CommandMeta.apply_command_and_get_size`
                // guarantee that `ptr` will point to a value of type `C`.
                let command = unsafe { ptr.read_unaligned::<C>() };
                command.write(world);
                std::mem::size_of::<C>()
            },
        };

        let old_len = self.bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        self.bytes
            .reserve(std::mem::size_of::<CommandMeta>() + std::mem::size_of::<C>());

        // Pointer to the bytes at the end of the buffer.
        // SAFETY: We know it is within bounds of the allocation, due to the call to `.reserve()`.
        let ptr = unsafe { self.bytes.as_mut_ptr().add(old_len) };

        // SAFETY: Due to the `.reserve()` call above, the end of the buffer has at least
        // enough space to fit a value of type `CommandMeta`.
        // Since the buffer is of type `MaybeUninit<u8>`, any byte patterns are valid.
        unsafe {
            ptr.cast::<CommandMeta>().write_unaligned(meta);
        }

        if std::mem::size_of::<C>() > 0 {
            // SAFETY: Due to the `.reserve()` call above, the buffer has enough space
            // to fit a value of type `C` after the metadata.
            // Since the buffer is of type `MaybeUninit<u8>`, any byte patterns are valid.
            // The value will eventually be dropped when `.apply()` is called.
            unsafe {
                ptr.add(std::mem::size_of::<CommandMeta>())
                    .cast::<C>()
                    .write_unaligned(command);
            }
        }

        // Extend the length of the buffer to include the data we just wrote.
        // SAFETY: The new length is guaranteed to fit in the vector's capacity,
        // due to the call to `.reserve()` above.
        unsafe {
            self.bytes
                .set_len(std::mem::size_of::<CommandMeta>() + std::mem::size_of::<C>() + old_len);
        }
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // Pointer that will iterate over the entries of the buffer.
        let mut cursor = self.bytes.as_mut_ptr();

        // The address of the end of the buffer.
        let end_addr = cursor as usize + self.bytes.len();

        // Reset the buffer so its internal storage can get reused.
        // SAFETY: Below, each command will have its ownership transferred to user code,
        // which will be responsible for dropping each value.
        unsafe { self.bytes.set_len(0) };

        while (cursor as usize) < end_addr {
            // SAFETY: The cursor is either at the start of the buffer, or just after the previous command.
            // Since we know that the cursor is in bounds, it must point to the start of a new command.
            let meta = unsafe { cursor.cast::<CommandMeta>().read_unaligned() };
            // Advance to the bytes just after `meta`, which represent a type-erased command.
            // SAFETY: For most types of `Command`, the pointer immediately following the metadata
            // is guaranteed to be in bounds. If the command is a zero-sized type (ZST), then the cursor
            // might be 1 byte past the end of the buffer, which is safe.
            cursor = unsafe { cursor.add(std::mem::size_of::<CommandMeta>()) };
            // Construct an owned pointer to the command.
            // SAFETY: Since the buffer has been cleared, the command won't get
            // observed later and we can transfer ownership from the buffer.
            let cmd = unsafe { OwningPtr::new(std::ptr::NonNull::new_unchecked(cursor.cast())) };
            // SAFETY: The data underneath the cursor must correspond to the type erased in metadata,
            // since they were stored next to each other by `.push()`.
            // For ZSTs, the type doesn't matter as long as the pointer is non-null.
            let size = unsafe { (meta.apply_command_and_get_size)(cmd, world) };
            // Advance the cursor past the command.
            // For ZSTs, the cursor will not move.
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
