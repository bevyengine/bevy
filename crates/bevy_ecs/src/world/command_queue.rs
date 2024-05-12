use crate::system::{SystemBuffer, SystemMeta};

use std::{fmt::Debug, mem::MaybeUninit};

use bevy_ptr::{OwningPtr, Unaligned};
use bevy_utils::tracing::warn;

use crate::world::{Command, World};

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    ///
    /// `world` is optional to allow this one function pointer to perform double-duty as a drop.
    ///
    /// Returns the size of `T` in bytes.
    consume_command_and_get_size:
        unsafe fn(value: OwningPtr<Unaligned>, world: Option<&mut World>) -> usize,
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

// CommandQueue needs to implement Debug manually, rather than deriving it, because the derived impl just prints
// [core::mem::maybe_uninit::MaybeUninit<u8>, core::mem::maybe_uninit::MaybeUninit<u8>, ..] for every byte in the vec,
// which gets extremely verbose very quickly, while also providing no useful information.
// It is not possible to soundly print the values of the contained bytes, as some of them may be padding or uninitialized (#4863)
// So instead, the manual impl just prints the length of vec.
impl Debug for CommandQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandQueue")
            .field("len_bytes", &self.bytes.len())
            .finish_non_exhaustive()
    }
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
        // Stores a command alongside its metadata.
        // `repr(C)` prevents the compiler from reordering the fields,
        // while `repr(packed)` prevents the compiler from inserting padding bytes.
        #[repr(C, packed)]
        struct Packed<T: Command> {
            meta: CommandMeta,
            command: T,
        }

        let meta = CommandMeta {
            consume_command_and_get_size: |command, world| {
                // SAFETY: According to the invariants of `CommandMeta.consume_command_and_get_size`,
                // `command` must point to a value of type `C`.
                let command: C = unsafe { command.read_unaligned() };
                match world {
                    // Apply command to the provided world...
                    Some(world) => command.apply(world),
                    // ...or discard it.
                    None => drop(command),
                }
                std::mem::size_of::<C>()
            },
        };

        let old_len = self.bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        self.bytes.reserve(std::mem::size_of::<Packed<C>>());

        // Pointer to the bytes at the end of the buffer.
        // SAFETY: We know it is within bounds of the allocation, due to the call to `.reserve()`.
        let ptr = unsafe { self.bytes.as_mut_ptr().add(old_len) };

        // Write the metadata into the buffer, followed by the command.
        // We are using a packed struct to write them both as one operation.
        // SAFETY: `ptr` must be non-null, since it is within a non-null buffer.
        // The call to `reserve()` ensures that the buffer has enough space to fit a value of type `C`,
        // and it is valid to write any bit pattern since the underlying buffer is of type `MaybeUninit<u8>`.
        unsafe {
            ptr.cast::<Packed<C>>()
                .write_unaligned(Packed { meta, command });
        }

        // Extend the length of the buffer to include the data we just wrote.
        // SAFETY: The new length is guaranteed to fit in the vector's capacity,
        // due to the call to `.reserve()` above.
        unsafe {
            self.bytes
                .set_len(old_len + std::mem::size_of::<Packed<C>>());
        }
    }

    /// Execute the queued [`Command`]s in the world after applying any commands in the world's internal queue.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush_entities();

        self.apply_or_drop_queued(Some(world));
    }

    /// If `world` is [`Some`], this will apply the queued [commands](`Command`).
    /// If `world` is [`None`], this will drop the queued [commands](`Command`) (without applying them).
    /// This clears the queue.
    #[inline]
    fn apply_or_drop_queued(&mut self, mut world: Option<&mut World>) {
        // The range of pointers of the filled portion of `self.bytes`.
        let bytes_range = self.bytes.as_mut_ptr_range();

        // Pointer that will iterate over the entries of the buffer.
        let cursor = bytes_range.start;

        let end = bytes_range.end;

        // Reset the buffer, so it can be reused after this function ends.
        // In the loop below, ownership of each command will be transferred into user code.
        // SAFETY: `set_len(0)` is always valid.
        unsafe { self.bytes.set_len(0) };

        // Create a stack for the command queue's we will be applying as commands may queue additional commands.
        // This is preferred over recursion to avoid stack overflows.
        let mut resolving_commands = vec![(cursor, end)];
        // Take ownership of any additional buffers so they are not free'd uintil they are iterated.
        let mut buffers = Vec::new();

        // Add any commands in the world's internal queue to the top of the stack.
        if let Some(world) = &mut world {
            if !world.command_queue.is_empty() {
                let mut bytes = std::mem::take(&mut world.command_queue.bytes);
                let bytes_range = bytes.as_mut_ptr_range();
                resolving_commands.push((bytes_range.start, bytes_range.end));
                buffers.push(bytes);
            }
        }

        while let Some((mut cursor, mut end)) = resolving_commands.pop() {
            while cursor < end {
                // SAFETY: The cursor is either at the start of the buffer, or just after the previous command.
                // Since we know that the cursor is in bounds, it must point to the start of a new command.
                let meta = unsafe { cursor.cast::<CommandMeta>().read_unaligned() };
                // Advance to the bytes just after `meta`, which represent a type-erased command.
                // SAFETY: For most types of `Command`, the pointer immediately following the metadata
                // is guaranteed to be in bounds. If the command is a zero-sized type (ZST), then the cursor
                // might be 1 byte past the end of the buffer, which is safe.
                cursor = unsafe { cursor.add(std::mem::size_of::<CommandMeta>()) };
                // Construct an owned pointer to the command.
                // SAFETY: It is safe to transfer ownership out of `self.bytes`, since the call to `set_len(0)` above
                // guarantees that nothing stored in the buffer will get observed after this function ends.
                // `cmd` points to a valid address of a stored command, so it must be non-null.
                let cmd = unsafe {
                    OwningPtr::<Unaligned>::new(std::ptr::NonNull::new_unchecked(cursor.cast()))
                };
                // SAFETY: The data underneath the cursor must correspond to the type erased in metadata,
                // since they were stored next to each other by `.push()`.
                // For ZSTs, the type doesn't matter as long as the pointer is non-null.
                let size =
                    unsafe { (meta.consume_command_and_get_size)(cmd, world.as_deref_mut()) };
                // Advance the cursor past the command. For ZSTs, the cursor will not move.
                // At this point, it will either point to the next `CommandMeta`,
                // or the cursor will be out of bounds and the loop will end.
                // SAFETY: The address just past the command is either within the buffer,
                // or 1 byte past the end, so this addition will not overflow the pointer's allocation.
                cursor = unsafe { cursor.add(size) };

                if let Some(world) = &mut world {
                    // If the command we just applied generated more commands we must apply those first
                    if !world.command_queue.is_empty() {
                        // If our current list of commands isn't complete push it to the `resolving_commands` stack to be applied after
                        if cursor < end {
                            resolving_commands.push((cursor, end));
                        }
                        let mut bytes = std::mem::take(&mut world.command_queue.bytes);

                        // Start applying the new queue
                        let bytes_range = bytes.as_mut_ptr_range();
                        cursor = bytes_range.start;
                        end = bytes_range.end;

                        // Store our buffer so it is not dropped;
                        buffers.push(bytes);
                    }
                }
            }
            // Re-use last buffer to avoid re-allocation
            if let (Some(world), Some(buffer)) = (&mut world, buffers.pop()) {
                world.command_queue.bytes = buffer;
                // SAFETY: `set_len(0)` is always valid.
                unsafe { world.command_queue.bytes.set_len(0) };
            }
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty
    pub fn append(&mut self, other: &mut CommandQueue) {
        self.bytes.append(&mut other.bytes);
    }

    /// Returns false if there are any commands in the queue
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        if !self.bytes.is_empty() {
            warn!("CommandQueue has un-applied commands being dropped.");
        }
        self.apply_or_drop_queued(None);
    }
}

impl SystemBuffer for CommandQueue {
    #[inline]
    fn apply(&mut self, _system_meta: &SystemMeta, world: &mut World) {
        #[cfg(feature = "trace")]
        let _span_guard = _system_meta.commands_span.enter();
        self.apply(world);
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
        fn apply(self, _: &mut World) {}
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

    /// Asserts that inner [commands](`Command`) are dropped on early drop of [`CommandQueue`].
    /// Originally identified as an issue in [#10676](https://github.com/bevyengine/bevy/issues/10676)
    #[test]
    fn test_command_queue_inner_drop_early() {
        let mut queue = CommandQueue::default();

        let (dropcheck_a, drops_a) = DropCheck::new();
        let (dropcheck_b, drops_b) = DropCheck::new();

        queue.push(dropcheck_a);
        queue.push(dropcheck_b);

        assert_eq!(drops_a.load(Ordering::Relaxed), 0);
        assert_eq!(drops_b.load(Ordering::Relaxed), 0);

        drop(queue);

        assert_eq!(drops_a.load(Ordering::Relaxed), 1);
        assert_eq!(drops_b.load(Ordering::Relaxed), 1);
    }

    struct SpawnCommand;

    impl Command for SpawnCommand {
        fn apply(self, world: &mut World) {
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
    #[allow(dead_code)]
    struct PanicCommand(String);
    impl Command for PanicCommand {
        fn apply(self, _: &mut World) {
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

    #[allow(dead_code)]
    struct CommandWithPadding(u8, u16);
    impl Command for CommandWithPadding {
        fn apply(self, _: &mut World) {}
    }

    #[cfg(miri)]
    #[test]
    fn test_uninit_bytes() {
        let mut queue = CommandQueue::default();
        queue.push(CommandWithPadding(0, 0));
        let _ = format!("{:?}", queue.bytes);
    }
}
