use std::mem::{ManuallyDrop, MaybeUninit};

use super::Command;
use crate::world::World;

struct CommandMeta {
    offset: usize,
    func: unsafe fn(value: *mut MaybeUninit<u8>, world: &mut World),
}

/// A queue of [`Command`]s
//
// NOTE: [`CommandQueue`] is implemented via a `Vec<MaybeUninit<u8>>` over a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
#[derive(Default)]
pub struct CommandQueue {
    bytes: Vec<MaybeUninit<u8>>,
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
        /// SAFETY: This function is only every called when the `command` bytes is the associated
        /// [`Commands`] `T` type. Also this only reads the data via `read_unaligned` so unaligned
        /// accesses are safe.
        unsafe fn write_command<T: Command>(command: *mut MaybeUninit<u8>, world: &mut World) {
            let command = command.cast::<T>().read_unaligned();
            command.write(world);
        }

        let size = std::mem::size_of::<C>();
        let old_len = self.bytes.len();

        self.metas.push(CommandMeta {
            offset: old_len,
            func: write_command::<C>,
        });

        // Use `ManuallyDrop` to forget `command` right away, avoiding
        // any use of it after the `ptr::copy_nonoverlapping`.
        let command = ManuallyDrop::new(command);

        if size > 0 {
            self.bytes.reserve(size);

            // SAFETY: The internal `bytes` vector has enough storage for the
            // command (see the call the `reserve` above), the vector has
            // its length set appropriately and can contain any kind of bytes.
            // In case we're writing a ZST and the `Vec` hasn't allocated yet
            // then `as_mut_ptr` will be a dangling (non null) pointer, and
            // thus valid for ZST writes.
            // Also `command` is forgotten so that  when `apply` is called
            // later, a double `drop` does not occur.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &*command as *const C as *const MaybeUninit<u8>,
                    self.bytes.as_mut_ptr().add(old_len),
                    size,
                );
                self.bytes.set_len(old_len + size);
            }
        }
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // SAFETY: In the iteration below, `meta.func` will safely consume and drop each pushed command.
        // This operation is so that we can reuse the bytes `Vec<u8>`'s internal storage and prevent
        // unnecessary allocations.
        unsafe { self.bytes.set_len(0) };

        for meta in self.metas.drain(..) {
            // SAFETY: The implementation of `write_command` is safe for the according Command type.
            // It's ok to read from `bytes.as_mut_ptr()` because we just wrote to it in `push`.
            // The bytes are safely cast to their original type, safely read, and then dropped.
            unsafe {
                (meta.func)(self.bytes.as_mut_ptr().add(meta.offset), world);
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
            world.spawn();
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
