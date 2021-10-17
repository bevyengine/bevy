use std::marker::PhantomData;

use super::{Command, IteratorCommand};
use crate::world::World;

struct CommandMeta {
    offset: usize,
    iter_item_count: usize,
    func: unsafe fn(value: *mut u8, world: &mut World, iter_item_count: usize),
}

/// A queue of [`Command`]s
//
// NOTE: [`CommandQueue`] is implemented via a `Vec<u8>` over a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
#[derive(Default)]
pub struct CommandQueue {
    bytes: Vec<u8>,
    metas: Vec<CommandMeta>,
}

// SAFE: All commands [`Command`] implement [`Send`]
unsafe impl Send for CommandQueue {}

// SAFE: `&CommandQueue` never gives access to the inner commands.
unsafe impl Sync for CommandQueue {}

impl CommandQueue {
    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<C: Command>(&mut self, command: C) {
        self.push_with_iterator(std::iter::empty(), command);
    }

    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push_with_iterator<C, I>(&mut self, iterator: I, command: C)
    where
        C: IteratorCommand,
        I: IntoIterator<Item = C::IterItem>,
    {
        let iter = iterator.into_iter();

        let cmd_size = std::mem::size_of::<C>();
        let iter_item_size = std::mem::size_of::<C::IterItem>();
        let old_len = self.bytes.len();

        // The hints are never relied on for soundness.
        let (iter_min_hint, iter_max_hint) = iter.size_hint();
        let iter_item_count_hint = iter_max_hint.unwrap_or(iter_min_hint);

        // Use checked_add to guard against malicious iterator impls.
        let alloc_hint = cmd_size
            // no overflow checks here, as even if these overflow every item also has a separate reserve call.
            .checked_add(iter_item_count_hint * iter_item_size)
            .unwrap_or(cmd_size);
        self.bytes.reserve(alloc_hint);

        if cmd_size > 0 {
            // SAFE: The internal `bytes` vector has enough storage for the
            // command (see the call the `reserve` above), and the vector has
            // its length set appropriately.
            unsafe {
                self.bytes
                    .as_mut_ptr()
                    .add(self.bytes.len())
                    .cast::<C>()
                    .write_unaligned(command);
                self.bytes.set_len(self.bytes.len() + cmd_size);
            }
        }

        let mut iter_item_count = 0;
        if iter_item_size > 0 {
            for item in iter {
                self.bytes.reserve(iter_item_size);
                // SAFE: The internal `bytes` vector has enough storage for the
                // command (see the call the `reserve` above), and the vector has
                // its length set appropriately.
                unsafe {
                    self.bytes
                        .as_mut_ptr()
                        .add(self.bytes.len())
                        .cast::<C::IterItem>()
                        .write_unaligned(item);

                    self.bytes.set_len(self.bytes.len() + iter_item_size);
                }
                iter_item_count += 1;
            }
        } else {
            iter_item_count = iter.count();
        }

        /// SAFE: This function is only every called when the `command` bytes is the associated
        /// [`Commands`] `C` type. Also this only reads the data via `read_unaligned` so unaligned
        /// accesses are safe.
        unsafe fn write_command<C: IteratorCommand>(
            command: *mut u8,
            world: &mut World,
            iter_item_count: usize,
        ) {
            let read_base = command.add(std::mem::size_of::<C>());
            let indexes = 0..iter_item_count;
            let command = command.cast::<C>().read_unaligned();

            struct ByteReadIterator<T: Send + Sync + 'static> {
                indexes: std::ops::Range<usize>,
                read_base: *mut u8,
                _marker: PhantomData<T>,
            }

            impl<T: Send + Sync + 'static> Iterator for ByteReadIterator<T> {
                type Item = T;

                fn size_hint(&self) -> (usize, Option<usize>) {
                    self.indexes.size_hint()
                }

                fn next(&mut self) -> Option<Self::Item> {
                    unsafe {
                        self.indexes
                            .next()
                            .map(|index| self.read_base.cast::<T>().add(index).read_unaligned())
                    }
                }
            }

            impl<T: Send + Sync + 'static> std::iter::DoubleEndedIterator for ByteReadIterator<T> {
                fn next_back(&mut self) -> Option<Self::Item> {
                    unsafe {
                        self.indexes
                            .next_back()
                            .map(|index| self.read_base.cast::<T>().add(index).read_unaligned())
                    }
                }
            }

            impl<T: Send + Sync + 'static> std::iter::ExactSizeIterator for ByteReadIterator<T> {}

            impl<T: Send + Sync + 'static> std::iter::FusedIterator for ByteReadIterator<T> {}

            impl<T: Send + Sync + 'static> Drop for ByteReadIterator<T> {
                fn drop(&mut self) {
                    self.for_each(drop);
                }
            }

            command.write_with_iterator(
                world,
                ByteReadIterator {
                    read_base,
                    indexes,
                    _marker: PhantomData,
                },
            );
        }

        self.metas.push(CommandMeta {
            offset: old_len,
            iter_item_count,
            func: write_command::<C>,
        });
    }

    /// Execute the queued [`Command`]s in the world.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush();

        // SAFE: In the iteration below, `meta.func` will safely consume and drop each pushed command.
        // This operation is so that we can reuse the bytes `Vec<u8>`'s internal storage and prevent
        // unnecessary allocations.
        unsafe { self.bytes.set_len(0) };

        let byte_ptr = if self.bytes.as_mut_ptr().is_null() {
            // SAFE: If the vector's buffer pointer is `null` this mean nothing has been pushed to its bytes.
            // This means either that:
            //
            // 1) There are no commands so this pointer will never be read/written from/to.
            //
            // 2) There are only zero-sized commands pushed.
            //    According to https://doc.rust-lang.org/std/ptr/index.html
            //    "The canonical way to obtain a pointer that is valid for zero-sized accesses is NonNull::dangling"
            //    therefore it is safe to call `read_unaligned` on a pointer produced from `NonNull::dangling` for
            //    zero-sized commands.
            unsafe { std::ptr::NonNull::dangling().as_mut() }
        } else {
            self.bytes.as_mut_ptr()
        };

        for meta in self.metas.drain(..) {
            // SAFE: The implementation of `write_command` is safe for the according Command type.
            // The bytes are safely cast to their original type, safely read, and then dropped.
            unsafe {
                (meta.func)(byte_ptr.add(meta.offset), world, meta.iter_item_count);
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
}
