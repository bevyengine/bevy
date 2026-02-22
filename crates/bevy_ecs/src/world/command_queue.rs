use crate::{
    change_detection::MaybeLocation,
    system::{Command, SystemBuffer, SystemMeta},
    world::{DeferredWorld, World},
};

use alloc::{boxed::Box, vec::Vec};
use bevy_ptr::{OwningPtr, Unaligned};
use core::{
    cell::UnsafeCell,
    fmt::Debug,
    mem::{size_of, MaybeUninit},
    panic::AssertUnwindSafe,
    ptr::NonNull,
};
use log::warn;

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    ///
    /// `world` is optional to allow this one function pointer to perform double-duty as a drop.
    ///
    /// Advances `cursor` by the size of `T` in bytes.
    consume_command_and_get_size:
        unsafe fn(value: OwningPtr<Unaligned>, world: Option<NonNull<World>>, cursor: &mut usize),
}

/// Densely and efficiently stores a queue of heterogenous types implementing [`Command`].
// NOTE: [`CommandQueue`] is implemented via a `Vec<MaybeUninit<u8>>` instead of a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
pub struct CommandQueue {
    // This buffer densely stores all queued commands.
    //
    // For each command, one `CommandMeta` is stored, followed by zero or more bytes
    // to store the command itself. To interpret these bytes, a pointer must
    // be passed to the corresponding `CommandMeta.apply_command_and_get_size` fn pointer.
    pub(crate) bytes: UnsafeCell<Vec<MaybeUninit<u8>>>,
    pub(crate) cursor: UnsafeCell<usize>,
    pub(crate) panic_recovery: UnsafeCell<Vec<MaybeUninit<u8>>>,
    pub(crate) caller: MaybeLocation,
}

impl Default for CommandQueue {
    #[track_caller]
    fn default() -> Self {
        Self {
            bytes: Default::default(),
            cursor: Default::default(),
            panic_recovery: Default::default(),
            caller: MaybeLocation::caller(),
        }
    }
}

/// Wraps pointers to a [`CommandQueue`], used internally to avoid stacked borrow rules when
/// partially applying the world's command queue recursively
#[derive(Clone)]
pub(crate) struct RawCommandQueue {
    pub(crate) bytes: NonNull<Vec<MaybeUninit<u8>>>,
    pub(crate) cursor: NonNull<usize>,
    pub(crate) panic_recovery: NonNull<Vec<MaybeUninit<u8>>>,
}

// CommandQueue needs to implement Debug manually, rather than deriving it, because the derived impl of
// UnsafeCell doesn't print its contents.
// The manual impl just prints the caller.
impl Debug for CommandQueue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CommandQueue")
            .field("caller", &self.caller)
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
    pub fn push(&mut self, command: impl Command) {
        // SAFETY: self is guaranteed to live for the lifetime of this method
        unsafe {
            self.get_raw().push(command);
        }
    }

    /// Execute the queued [`Command`]s in the world after applying any commands in the world's internal queue.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the world's internal queue
        world.flush_commands();

        // SAFETY: A reference is always a valid pointer
        unsafe {
            self.get_raw().apply_or_drop_queued(Some(world.into()));
        }
    }

    /// Take all commands from `other` and append them to `self`, leaving `other` empty
    pub fn append(&mut self, other: &mut CommandQueue) {
        self.bytes.get_mut().append(other.bytes.get_mut());
    }

    /// Returns false if there are any commands in the queue
    #[inline]
    pub fn is_empty(&self) -> bool {
        // SAFETY: aliasing rules are upheld by `RawCommandQueue`s if they exist
        unsafe { *self.cursor.get() >= (&*self.bytes.get()).len() }
    }

    /// Returns a [`RawCommandQueue`] instance sharing the underlying command queue.
    ///
    /// # Safety
    /// Caller must ensure that the `RawCommandQueue` is not used mutably at the same time
    /// as `self` or any other raw command queues created from `self`.
    pub(crate) unsafe fn get_raw(&self) -> RawCommandQueue {
        // SAFETY: self is always valid memory and caller upholds mutability requirement
        unsafe {
            RawCommandQueue {
                bytes: NonNull::new_unchecked(self.bytes.get()),
                cursor: NonNull::new_unchecked(self.cursor.get()),
                panic_recovery: NonNull::new_unchecked(self.panic_recovery.get()),
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
                panic_recovery: NonNull::new_unchecked(Box::into_raw(Box::default())),
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
    pub unsafe fn push<C: Command>(&mut self, command: C) {
        // Stores a command alongside its metadata.
        // `repr(C)` prevents the compiler from reordering the fields,
        // while `repr(packed)` prevents the compiler from inserting padding bytes.
        #[repr(C, packed)]
        struct Packed<C: Command> {
            meta: CommandMeta,
            command: C,
        }

        let meta = CommandMeta {
            consume_command_and_get_size: |command, world, cursor| {
                *cursor += size_of::<C>();

                // SAFETY: According to the invariants of `CommandMeta.consume_command_and_get_size`,
                // `command` must point to a value of type `C`.
                let command: C = unsafe { command.read_unaligned() };
                match world {
                    // Apply command to the provided world...
                    Some(mut world) => {
                        // SAFETY: Caller ensures pointer is not null
                        let world = unsafe { world.as_mut() };
                        command.apply(world);
                        // The command may have queued up world commands, which we flush here to ensure they are also picked up.
                        // If the current command queue already the World Command queue, this will still behave appropriately because the global cursor
                        // is still at the current `stop`, ensuring only the newly queued Commands will be applied.
                        world.flush();
                    }
                    // ...or discard it.
                    None => drop(command),
                }
            },
        };

        // SAFETY: There are no outstanding references to self.bytes
        let bytes = unsafe { self.bytes.as_mut() };

        let old_len = bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        bytes.reserve(size_of::<Packed<C>>());

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
            bytes.set_len(old_len + size_of::<Packed<C>>());
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
        let start = *self.cursor.as_ref();
        let stop = (*self.bytes.as_ref()).len();
        let mut local_cursor = start;
        // SAFETY: we are setting the global cursor to the current length to prevent the executing commands from applying
        // the remaining commands currently in this list. This is safe.
        *self.cursor.as_mut() = stop;

        while local_cursor < stop {
            // SAFETY: The cursor is either at the start of the buffer, or just after the previous command.
            // Since we know that the cursor is in bounds, it must point to the start of a new command.
            let meta = unsafe {
                self.bytes
                    .as_mut()
                    .as_mut_ptr()
                    .add(local_cursor)
                    .cast::<CommandMeta>()
                    .read_unaligned()
            };

            // Advance to the bytes just after `meta`, which represent a type-erased command.
            local_cursor += size_of::<CommandMeta>();
            // Construct an owned pointer to the command.
            // SAFETY: It is safe to transfer ownership out of `self.bytes`, since the increment of `cursor` above
            // guarantees that nothing stored in the buffer will get observed after this function ends.
            // `cmd` points to a valid address of a stored command, so it must be non-null.
            let cmd = unsafe {
                OwningPtr::<Unaligned>::new(NonNull::new_unchecked(
                    self.bytes.as_mut().as_mut_ptr().add(local_cursor).cast(),
                ))
            };
            let f = AssertUnwindSafe(|| {
                // SAFETY: The data underneath the cursor must correspond to the type erased in metadata,
                // since they were stored next to each other by `.push()`.
                // For ZSTs, the type doesn't matter as long as the pointer is non-null.
                // This also advances the cursor past the command. For ZSTs, the cursor will not move.
                // At this point, it will either point to the next `CommandMeta`,
                // or the cursor will be out of bounds and the loop will end.
                unsafe { (meta.consume_command_and_get_size)(cmd, world, &mut local_cursor) };
            });

            #[cfg(feature = "std")]
            {
                let result = std::panic::catch_unwind(f);

                if let Err(payload) = result {
                    // local_cursor now points to the location _after_ the panicked command.
                    // Add the remaining commands that _would have_ been applied to the
                    // panic_recovery queue.
                    //
                    // This uses `current_stop` instead of `stop` to account for any commands
                    // that were queued _during_ this panic.
                    //
                    // This is implemented in such a way that if apply_or_drop_queued() are nested recursively in,
                    // an applied Command, the correct command order will be retained.
                    let panic_recovery = self.panic_recovery.as_mut();
                    let bytes = self.bytes.as_mut();
                    let current_stop = bytes.len();
                    panic_recovery.extend_from_slice(&bytes[local_cursor..current_stop]);
                    bytes.set_len(start);
                    *self.cursor.as_mut() = start;

                    // This was the "top of the apply stack". If we are _not_ at the top of the apply stack,
                    // when we call`resume_unwind" the caller "closer to the top" will catch the unwind and do this check,
                    // until we reach the top.
                    if start == 0 {
                        bytes.append(panic_recovery);
                    }
                    std::panic::resume_unwind(payload);
                }
            }

            #[cfg(not(feature = "std"))]
            (f)();
        }

        // Reset the buffer: all commands past the original `start` cursor have been applied.
        // SAFETY: we are setting the length of bytes to the original length, minus the length of the original
        // list of commands being considered. All bytes remaining in the Vec are still valid, unapplied commands.
        unsafe {
            self.bytes.as_mut().set_len(start);
            *self.cursor.as_mut() = start;
        };
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        // SAFETY: aliasing rules are upheld by `RawCommandQueue`s if they exist
        let is_empty = unsafe { (&*self.bytes.get()).is_empty() };
        if !is_empty {
            if let Some(caller) = self.caller.into_option() {
                warn!("CommandQueue has un-applied commands being dropped. Did you forget to call SystemState::apply? caller:{caller:?}");
            } else {
                warn!("CommandQueue has un-applied commands being dropped. Did you forget to call SystemState::apply?");
            }
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

    #[inline]
    fn queue(&mut self, _system_meta: &SystemMeta, mut world: DeferredWorld) {
        world.commands().append(self);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{component::Component, resource::Resource};
    use alloc::{borrow::ToOwned, string::String, sync::Arc};
    use core::{
        panic::AssertUnwindSafe,
        sync::atomic::{AtomicU32, Ordering},
    };

    #[cfg(miri)]
    use alloc::format;

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

    #[derive(Component)]
    struct A;

    struct SpawnCommand;

    impl Command for SpawnCommand {
        fn apply(self, world: &mut World) {
            world.spawn(A);
        }
    }

    #[test]
    fn test_command_queue_inner() {
        let mut queue = CommandQueue::default();

        queue.push(SpawnCommand);
        queue.push(SpawnCommand);

        let mut world = World::new();
        queue.apply(&mut world);

        assert_eq!(world.query::<&A>().query(&world).count(), 2);

        // The previous call to `apply` cleared the queue.
        // This call should do nothing.
        queue.apply(&mut world);
        assert_eq!(world.query::<&A>().query(&world).count(), 2);
    }

    #[expect(
        dead_code,
        reason = "The inner string is used to ensure that, when the PanicCommand gets pushed to the queue, some data is written to the `bytes` vector."
    )]
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

        // Even though the first command panicked, it's still ok to push
        // more commands.
        queue.push(SpawnCommand);
        queue.push(SpawnCommand);
        queue.apply(&mut world);
        assert_eq!(world.query::<&A>().query(&world).count(), 3);
    }

    #[test]
    fn test_command_queue_inner_nested_panic_safe() {
        std::panic::set_hook(Box::new(|_| {}));

        #[derive(Resource, Default)]
        struct Order(Vec<usize>);

        let mut world = World::new();
        world.init_resource::<Order>();

        fn add_index(index: usize) -> impl Command {
            move |world: &mut World| world.resource_mut::<Order>().0.push(index)
        }
        world.commands().queue(add_index(1));
        world.commands().queue(|world: &mut World| {
            world.commands().queue(add_index(2));
            world.commands().queue(PanicCommand("I panic!".to_owned()));
            world.commands().queue(add_index(3));
            world.flush_commands();
        });
        world.commands().queue(add_index(4));

        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            world.flush_commands();
        }));

        world.commands().queue(add_index(5));
        world.flush_commands();
        assert_eq!(&world.resource::<Order>().0, &[1, 2, 3, 4, 5]);
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

    #[expect(
        dead_code,
        reason = "This struct is used to test how the CommandQueue reacts to padding added by rust's compiler."
    )]
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
