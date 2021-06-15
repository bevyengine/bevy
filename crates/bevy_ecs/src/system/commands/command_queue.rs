use super::Command;
use crate::world::World;

/// # Safety
///
/// This function is only every called when the `command` bytes is the associated
/// [`Commands`] `T` type. Also this only reads the data via `read_unaligned` so unaligned
/// accesses are safe.
unsafe fn invoke_command<T: Command>(command: *mut u8, world: &mut World) {
    let command = command.cast::<T>().read_unaligned();
    command.write(world);
}

struct CommandMeta {
    offset: usize,
    func: unsafe fn(value: *mut u8, world: &mut World),
}

#[derive(Default)]
pub(crate) struct CommandQueueInner {
    bytes: Vec<u8>,
    metas: Vec<CommandMeta>,
}

// SAFE: All commands [`Command`] implement [`Send`]
unsafe impl Send for CommandQueueInner {}

// SAFE: `&CommandQueueInner` never gives access to the inner commands.
unsafe impl Sync for CommandQueueInner {}

impl CommandQueueInner {
    /// Push a new `command` onto the queue.
    #[inline]
    pub fn push<C>(&mut self, command: C)
    where
        C: Command,
    {
        let size = std::mem::size_of::<C>();
        let old_len = self.bytes.len();

        self.metas.push(CommandMeta {
            offset: old_len,
            func: invoke_command::<C>,
        });

        if size > 0 {
            self.bytes.reserve(size);

            // SAFE: The internal `bytes` vector has enough storage for the
            // command (see the call the `reserve` above), and the vector has
            // its length set appropriately.
            // Also `command` is forgotten at the end of this function so that
            // when `apply` is called later, a double `drop` does not occur.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    &command as *const C as *const u8,
                    self.bytes.as_mut_ptr().add(old_len),
                    size,
                );
                self.bytes.set_len(old_len + size);
            }
        }

        std::mem::forget(command);
    }

    /// Invoke each command `func` for each inserted value with `world`
    /// and then clears the internal bytes/metas command vectors.
    pub fn apply(&mut self, world: &mut World) {
        let byte_ptr = self.bytes.as_mut_ptr();
        for meta in self.metas.iter() {
            // The implementation of `invoke_command` is safe for the according Command type.
            // The bytes are safely cast to their original type, safely read, and then dropped.
            unsafe {
                (meta.func)(byte_ptr.add(meta.offset), world);
            }
        }

        self.bytes.clear();
        self.metas.clear();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
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
        let mut queue = CommandQueueInner::default();

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
        let mut queue = CommandQueueInner::default();

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
}
