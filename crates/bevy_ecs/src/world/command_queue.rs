use crate::system::{SystemBuffer, SystemMeta};

use std::{
    fmt::Debug,
    mem::MaybeUninit,
    ptr::{addr_of_mut, NonNull},
};

use bevy_ptr::{OwningPtr, Unaligned};
use bevy_utils::tracing::warn;

use crate::world::{Command, World};

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    ///
    /// `world` is optional to allow this one function pointer to perform double-duty as a drop.
    ///
    /// Advances `cursor` by the size of `T` in bytes.
    consume_command_and_get_size: unsafe fn(
        value: OwningPtr<Unaligned>,
        world: Option<NonNull<World>>,
        cursor: NonNull<usize>,
    ),
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
    pub(crate) bytes: Vec<MaybeUninit<u8>>,
    pub(crate) cursor: usize,
}

/// Wraps pointers to a [`CommandQueue`], used internally to avoid stacked borrow rules when
/// partially applying the world's command queue recursively
#[derive(Clone)]
pub(crate) struct RawCommandQueue {
    pub(crate) bytes: NonNull<Vec<MaybeUninit<u8>>>,
    pub(crate) cursor: NonNull<usize>,
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
        // SAFETY: self is guaranteed to live for the lifetime of this method
        unsafe {
            self.get_raw().push(command);
        }
    }

    /// Execute the queued [`Command`]s in the world after applying any commands in the world's internal queue.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the previously queued entities
        world.flush_entities();

        // flush the world's internal queue
        world.flush_commands();

        // SAFETY: A reference is always a valid pointer
        unsafe {
            self.get_raw().apply_or_drop_queued(Some(world.into()));
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty
    pub fn append(&mut self, other: &mut CommandQueue) {
        self.bytes.append(&mut other.bytes);
    }

    /// Returns false if there are any commands in the queue
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.cursor >= self.bytes.len()
    }

    /// Returns a [`RawCommandQueue`] instance sharing the underlying command queue.
    pub(crate) fn get_raw(&mut self) -> RawCommandQueue {
        // SAFETY: self is always valid memory
        unsafe {
            RawCommandQueue {
                bytes: NonNull::new_unchecked(addr_of_mut!(self.bytes)),
                cursor: NonNull::new_unchecked(addr_of_mut!(self.cursor)),
            }
        }
    }
}

impl RawCommandQueue {
    /// Returns a new `RawCommandQueue` instance, this must be manually dropped.
    pub(crate) fn new() -> Self {
        // SAFETY: Pointers returned by `Box::into_raw` are guaranteed to be non null
        unsafe {
            Self {
                bytes: NonNull::new_unchecked(Box::into_raw(Box::default())),
                cursor: NonNull::new_unchecked(Box::into_raw(Box::new(0usize))),
            }
        }
    }

    /// Returns true if the queue is empty.
    ///
    /// # Safety
    ///
    /// * Caller ensures that `bytes` and `cursor` point to valid memory
    pub unsafe fn is_empty(&self) -> bool {
        // SAFETY: Pointers are guaranteed to be valid by requirements on `.clone_unsafe`
        (unsafe { *self.cursor.as_ref() }) >= (unsafe { self.bytes.as_ref() }).len()
    }

    /// Push a [`Command`] onto the queue.
    ///
    /// # Safety
    ///
    /// * Caller ensures that `self` has not outlived the underlying queue
    #[inline]
    pub unsafe fn push<C>(&mut self, command: C)
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
            consume_command_and_get_size: |command, world, mut cursor| {
                // SAFETY: Pointer is assured to be valid in `CommandQueue.apply_or_drop_queued`
                unsafe { *cursor.as_mut() += std::mem::size_of::<C>() }

                // SAFETY: According to the invariants of `CommandMeta.consume_command_and_get_size`,
                // `command` must point to a value of type `C`.
                let command: C = unsafe { command.read_unaligned() };
                match world {
                    // Apply command to the provided world...
                    // SAFETY: Calller ensures pointer is not null
                    Some(mut world) => command.apply(unsafe { world.as_mut() }),
                    // ...or discard it.
                    None => drop(command),
                }
            },
        };

        // SAFETY: There are no outstanding references to self.bytes
        let bytes = unsafe { self.bytes.as_mut() };

        let old_len = bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        bytes.reserve(std::mem::size_of::<Packed<C>>());

        // Pointer to the bytes at the end of the buffer.
        // SAFETY: We know it is within bounds of the allocation, due to the call to `.reserve()`.
        let ptr = unsafe { bytes.as_mut_ptr().add(old_len) };

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
            bytes.set_len(old_len + std::mem::size_of::<Packed<C>>());
        }
    }

    /// If `world` is [`Some`], this will apply the queued [commands](`Command`).
    /// If `world` is [`None`], this will drop the queued [commands](`Command`) (without applying them).
    /// This clears the queue.
    ///
    /// # Safety
    ///
    /// * Caller ensures that `self` has not outlived the underlying queue
    #[inline]
    pub(crate) unsafe fn apply_or_drop_queued(&mut self, world: Option<NonNull<World>>) {
        // SAFETY: If this is the command queue on world, world will not be dropped as we have a mutable reference
        // If this is not the command queue on world we have exclusive ownership and self will not be mutated
        while *self.cursor.as_ref() < self.bytes.as_ref().len() {
            // SAFETY: The cursor is either at the start of the buffer, or just after the previous command.
            // Since we know that the cursor is in bounds, it must point to the start of a new command.
            let meta = unsafe {
                self.bytes
                    .as_mut()
                    .as_mut_ptr()
                    .add(*self.cursor.as_ref())
                    .cast::<CommandMeta>()
                    .read_unaligned()
            };

            // Advance to the bytes just after `meta`, which represent a type-erased command.
            // SAFETY: For most types of `Command`, the pointer immediately following the metadata
            // is guaranteed to be in bounds. If the command is a zero-sized type (ZST), then the cursor
            // might be 1 byte past the end of the buffer, which is safe.
            unsafe { *self.cursor.as_mut() += std::mem::size_of::<CommandMeta>() };
            // Construct an owned pointer to the command.
            // SAFETY: It is safe to transfer ownership out of `self.bytes`, since the increment of `cursor` above
            // guarantees that nothing stored in the buffer will get observed after this function ends.
            // `cmd` points to a valid address of a stored command, so it must be non-null.
            let cmd = unsafe {
                OwningPtr::<Unaligned>::new(std::ptr::NonNull::new_unchecked(
                    self.bytes
                        .as_mut()
                        .as_mut_ptr()
                        .add(*self.cursor.as_ref())
                        .cast(),
                ))
            };
            // SAFETY: The data underneath the cursor must correspond to the type erased in metadata,
            // since they were stored next to each other by `.push()`.
            // For ZSTs, the type doesn't matter as long as the pointer is non-null.
            // This also advances the cursor past the command. For ZSTs, the cursor will not move.
            // At this point, it will either point to the next `CommandMeta`,
            // or the cursor will be out of bounds and the loop will end.
            unsafe { (meta.consume_command_and_get_size)(cmd, world, self.cursor) };
        }

        // Reset the buffer, so it can be reused after this function ends.
        // SAFETY: `set_len(0)` is always valid.
        unsafe {
            self.bytes.as_mut().set_len(0);
            *self.cursor.as_mut() = 0;
        };
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        if !self.bytes.is_empty() {
            warn!("CommandQueue has un-applied commands being dropped.");
        }
        // SAFETY: A reference is always a valid pointer
        unsafe { self.get_raw().apply_or_drop_queued(None) };
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
        // the cursor was incremented.
        assert!(queue.cursor > 0);

        // Even though the first command panicked, it's still ok to push
        // more commands.
        queue.push(SpawnCommand);
        queue.push(SpawnCommand);
        queue.apply(&mut world);
        assert_eq!(world.entities().len(), 3);
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
