use crate::{
    change_detection::MaybeLocation,
    system::{Command, SystemBuffer, SystemMeta},
    world::{DeferredWorld, World},
};

use alloc::vec::Vec;
use bevy_ptr::{OwningPtr, Unaligned};
use core::{
    fmt::Debug,
    mem::{size_of, MaybeUninit},
    ptr::NonNull,
};
use log::warn;

#[cfg(feature = "std")]
use crate::error::{BevyError, ErrorContext, Severity, PANIC_ORIGINATES_FROM_ERROR_HANDLER};
#[cfg(feature = "std")]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use bevy_utils::DebugName;
#[cfg(feature = "std")]
use std::{
    backtrace::Backtrace,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
};

struct CommandMeta {
    /// SAFETY: The `value` must point to a value of type `T: Command`,
    /// where `T` is some specific type that was used to produce this metadata.
    ///
    /// `world` is optional to allow this one function pointer to perform double-duty as a drop.
    ///
    /// Advances `cursor` by the size of `T` in bytes.
    consume_command_and_get_size:
        unsafe fn(value: OwningPtr<Unaligned>, world: Option<&mut World>, cursor: &mut usize),
}

/// Densely and efficiently stores a queue of heterogenous types implementing [`Command`].
// NOTE: [`CommandQueue`] is implemented via a `Vec<MaybeUninit<u8>>` instead of a `Vec<Box<dyn Command>>`
// as an optimization. Since commands are used frequently in systems as a way to spawn
// entities/components/resources, and it's not currently possible to parallelize these
// due to mutable [`World`] access, maximizing performance for [`CommandQueue`] is
// preferred to simplicity of implementation.
pub struct CommandQueue {
    /// This buffer densely stores all queued commands.
    ///
    /// For each command, one `CommandMeta` is stored, followed by zero or more bytes
    /// to store the command itself. To interpret these bytes, a pointer must
    /// be passed to the corresponding `CommandMeta.apply_command_and_get_size` fn pointer.
    pub(crate) bytes: Vec<MaybeUninit<u8>>,
    pub(crate) caller: MaybeLocation,
    /// Always emit a warning if a command is dropped before it is applied.
    /// Defaults to `true`.
    ///
    /// This setting can be turned off for commands that might be dropped (due to application exit) before those
    /// commands are applied in ordinary situations, for example delayed commands.
    warn_on_unapplied: bool,
}

impl Default for CommandQueue {
    #[track_caller]
    fn default() -> Self {
        Self {
            bytes: Default::default(),
            caller: MaybeLocation::caller(),
            warn_on_unapplied: true,
        }
    }
}

// CommandQueue needs to implement Debug manually, rather than deriving it, because the derived impl just prints
// [core::mem::maybe_uninit::MaybeUninit<u8>, core::mem::maybe_uninit::MaybeUninit<u8>, ..] for every byte in the vec,
// which gets extremely verbose very quickly, while also providing no useful information.
// It is not possible to soundly print the values of the contained bytes, as some of them may be padding or uninitialized (#4863)
// So instead, the manual impl just prints the length of vec.
impl Debug for CommandQueue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CommandQueue")
            .field("len_bytes", &self.bytes.len())
            .field("caller", &self.caller)
            .finish_non_exhaustive()
    }
}

// SAFETY: All commands [`Command`] implement [`Send`]
unsafe impl Send for CommandQueue {}

// SAFETY: `&CommandQueue` never gives access to the inner commands.
unsafe impl Sync for CommandQueue {}

impl CommandQueue {
    /// Create a queue that does not warn when dropped.
    #[track_caller]
    pub fn silent() -> Self {
        CommandQueue {
            bytes: Default::default(),
            caller: MaybeLocation::caller(),
            warn_on_unapplied: false,
        }
    }

    /// Push a [`Command`] onto the queue.
    #[inline]
    pub fn push<C: Command<Out = ()>>(&mut self, command: C) {
        // Stores a command alongside its metadata.
        // `repr(C)` prevents the compiler from reordering the fields,
        // while `repr(packed)` prevents the compiler from inserting padding bytes.
        #[repr(C, packed)]
        struct Packed<C: Command<Out = ()>> {
            meta: CommandMeta,
            command: C,
        }

        let meta = CommandMeta {
            consume_command_and_get_size: |command, mut world, cursor| {
                *cursor += size_of::<C>();

                // Putting the command onto the stack is necessary not just for alignment and to be able to consume it,
                // but also because applying the command may cause the command queue to reallocate.
                // SAFETY: According to the invariants of `CommandMeta.consume_command_and_get_size`,
                // `command` must point to a value of type `C`.
                let command: C = unsafe { command.read_unaligned() };

                let f = || {
                    match world.as_deref_mut() {
                        // Apply command to the provided world...
                        Some(world) => {
                            command.apply(world);
                            // The command may have queued up world commands, which we flush here to ensure they are also picked up.
                            // If the current command queue already the World Command queue, this will still behave appropriately because the global cursor
                            // is still at the current `stop`, ensuring only the newly queued Commands will be applied.
                            world.flush();
                        }
                        // ...or discard it.
                        None => drop(command),
                    }
                };

                #[cfg(feature = "std")]
                {
                    let result = catch_unwind(AssertUnwindSafe(f));
                    if let Err(payload) = result {
                        let name = DebugName::type_name::<C>();
                        handle_panic_payload(world, payload, name);
                    }
                }

                #[cfg(not(feature = "std"))]
                (f)();
            },
        };

        let old_len = self.bytes.len();

        // Reserve enough bytes for both the metadata and the command itself.
        self.bytes.reserve(size_of::<Packed<C>>());

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
            self.bytes.set_len(old_len + size_of::<Packed<C>>());
        }
    }

    /// Execute the queued [`Command`]s in the world after applying any commands in the world's internal queue.
    /// This clears the queue.
    #[inline]
    pub fn apply(&mut self, world: &mut World) {
        // flush the world's internal queue
        world.flush_commands();
        // SAFETY:
        // * `self` is always returned
        // * The first command always start at 0
        // * `&mut self` prevents all other access to this queue
        let mut runner = unsafe { CommandQueueRunner::new((self, world), |(queue, _)| queue, 0) };
        runner.run(|(_, world)| Some(world));
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

    /// The number of bytes of commands in the queue.
    pub(crate) fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Silences drop warning if commands are unapplied.
    pub fn silence_drop_warning(&mut self) {
        self.warn_on_unapplied = false;
    }
}

impl Drop for CommandQueue {
    fn drop(&mut self) {
        if !self.bytes.is_empty() && self.warn_on_unapplied {
            if let Some(caller) = self.caller.into_option() {
                warn!("CommandQueue has un-applied commands being dropped. Did you forget to call SystemState::apply? caller:{caller:?}");
            } else {
                warn!("CommandQueue has un-applied commands being dropped. Did you forget to call SystemState::apply?");
            }
        }
        // Dropping a `CommandQueueRunner` will drop all unapplied commands.
        // SAFETY:
        // * `self` is always returned
        // * The first command always start at 0
        // * `&mut self` prevents all other access to this queue
        unsafe { drop(CommandQueueRunner::new(self, |queue| queue, 0)) };
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

/// A RAII guard used while running commands to ensure
/// that unapplied commands are dropped during unwind.
pub(crate) struct CommandQueueRunner<D, F>
where
    F: Fn(&mut D) -> &mut CommandQueue,
{
    data: D,
    command_queue: F,
    local_cursor: usize,
    start: usize,
    stop: usize,
}

impl<D, F> CommandQueueRunner<D, F>
where
    F: Fn(&mut D) -> &mut CommandQueue,
{
    /// Constructs a new [`CommandQueueRunner`] for the given queue.
    ///
    /// This runs commands from `start` to the end of the queue.
    ///
    /// To support running commands from the queue owned by the [`World`],
    /// this stores references to the [`World`] and/or a [`CommandQueue`] as opaque `data`,
    /// and uses the `command_queue` function to extract the reference to the queue.
    ///
    /// # Safety
    ///
    /// * `command_queue(&mut data)` must always return the same queue.
    /// * `start` is the index of the first byte of a command in the queue,
    ///   or the length of the queue
    /// * Until the `CommandQueueRunner` is dropped, nothing else may
    ///   access commands between `start` and `command_queue.len()`
    pub unsafe fn new(mut data: D, command_queue: F, start: usize) -> Self {
        let stop = command_queue(&mut data).len();
        Self {
            data,
            command_queue,
            local_cursor: start,
            start,
            stop,
        }
    }

    pub fn run(&mut self, world: impl Fn(&mut D) -> Option<&mut World>) {
        #[cfg(feature = "std")]
        {
            PANIC_ORIGINATES_FROM_ERROR_HANDLER.set(false);
        }

        while self.local_cursor < self.stop {
            let command_queue = (self.command_queue)(&mut self.data);

            // We must re-read the pointer to the allocation before each command
            // as the previous might have cause a reallocation.
            // SAFETY: The cursor is either at the start of the buffer, or just after the previous command.
            // Since we know that the cursor is in bounds, it must point to the start of a new command.
            let meta = unsafe {
                command_queue
                    .bytes
                    .as_mut_ptr()
                    .add(self.local_cursor)
                    .cast::<CommandMeta>()
                    .read_unaligned()
            };

            // Advance to the bytes just after `meta`, which represent a type-erased command.
            self.local_cursor += size_of::<CommandMeta>();
            // Construct an owned pointer to the command.
            // SAFETY: It is safe to transfer ownership out of `self.bytes`, since the increment of `cursor` above
            // guarantees that nothing stored in the buffer will get observed after this function ends.
            // `cmd` points to a valid address of a stored command, so it must be non-null.
            let cmd = unsafe {
                OwningPtr::<Unaligned>::new(NonNull::new_unchecked(
                    command_queue
                        .bytes
                        .as_mut_ptr()
                        .add(self.local_cursor)
                        .cast(),
                ))
            };
            // SAFETY: The data underneath the cursor must correspond to the type erased in metadata,
            // since they were stored next to each other by `.push()`.
            // For ZSTs, the type doesn't matter as long as the pointer is non-null.
            // This also advances the cursor past the command. For ZSTs, the cursor will not move.
            // At this point, it will either point to the next `CommandMeta`,
            // or the cursor will be out of bounds and the loop will end.
            unsafe {
                (meta.consume_command_and_get_size)(
                    cmd,
                    world(&mut self.data),
                    &mut self.local_cursor,
                );
            }
        }
    }
}

/// Handle a panic thrown within a command.
///
/// This is a separate non-generic function so that the panic handling code
/// is not monomorphized separately for each command type.
#[cfg(feature = "std")]
#[cold]
fn handle_panic_payload(
    world: Option<&mut World>,
    payload: Box<dyn core::any::Any + Send>,
    name: DebugName,
) {
    let panic_originates_from_error_handler = PANIC_ORIGINATES_FROM_ERROR_HANDLER.replace(false);
    if panic_originates_from_error_handler {
        resume_unwind(payload)
    }
    let Some(world) = world else {
        resume_unwind(payload)
    };
    let error =
        BevyError::new_with_backtrace(Severity::Panic, "Command panicked", Backtrace::disabled());
    world.fallback_error_handler()(error, ErrorContext::Command { name });
}

impl<D, F> Drop for CommandQueueRunner<D, F>
where
    F: Fn(&mut D) -> &mut CommandQueue,
{
    fn drop(&mut self) {
        // Drop any unapplied commands before resetting the length.
        // If `run` completed successfully then this will do nothing.
        self.run(|_| None);

        let command_queue = (self.command_queue)(&mut self.data);

        // Reset the buffer: all commands past the original `start` cursor have been applied.
        // SAFETY: we are setting the length of bytes to the original length, minus the length of the original
        // list of commands being considered. All bytes remaining in the Vec are still valid, unapplied commands.
        unsafe { command_queue.bytes.set_len(self.start) };
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        component::Component,
        error::{BevyError, ErrorContext, FallbackErrorHandler},
        resource::Resource,
    };
    use alloc::{
        borrow::ToOwned,
        string::{String, ToString},
        sync::Arc,
    };
    use core::{
        panic::AssertUnwindSafe,
        sync::atomic::{AtomicU32, Ordering},
    };
    use std::sync::Mutex;

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
        type Out = ();

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
        type Out = ();

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
        type Out = ();

        fn apply(self, _: &mut World) {
            panic!("command is panicking");
        }
    }

    #[test]
    fn test_command_queue_inner_panic_safe_panic() {
        let mut queue = CommandQueue::default();

        queue.push(PanicCommand("I panic!".to_owned()));
        // This will get skipped due to the panic
        queue.push(SpawnCommand);

        let mut world = World::new();

        let _ = catch_unwind(AssertUnwindSafe(|| {
            queue.apply(&mut world);
        }));

        // Even though the first command panicked, it's still ok to push
        // more commands.
        queue.push(SpawnCommand);
        queue.push(SpawnCommand);
        queue.apply(&mut world);
        assert_eq!(world.query::<&A>().query(&world).count(), 2);
    }

    #[test]
    fn test_command_queue_inner_panic_safe_handled() {
        let mut queue = CommandQueue::default();

        queue.push(PanicCommand("I panic!".to_owned()));
        // This will get run because the fallback error handler
        // handles the panicking command.
        queue.push(SpawnCommand);

        fn record_last_error(error: BevyError, context: ErrorContext) {
            *LAST_ERROR.lock().unwrap() = Some((error, context));
        }
        static LAST_ERROR: Mutex<Option<(BevyError, ErrorContext)>> = Mutex::new(None);
        *LAST_ERROR.lock().unwrap() = None;

        let mut world = World::new();
        world.insert_resource(FallbackErrorHandler(record_last_error));

        queue.apply(&mut world);

        // Even though the first command panicked, it's still ok to push
        // more commands.
        queue.push(SpawnCommand);
        queue.push(SpawnCommand);
        queue.apply(&mut world);
        assert_eq!(world.query::<&A>().query(&world).count(), 3);

        let (error, context) = LAST_ERROR.lock().unwrap().take().unwrap();
        assert!(error.to_string().contains("Command panicked"));
        let name = DebugName::type_name::<PanicCommand>();
        assert_eq!(context, ErrorContext::Command { name });
    }

    #[test]
    fn test_command_queue_inner_nested_panic_safe_panic() {
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
            // Everything after here will get skipped due to the panic
            world.commands().queue(add_index(3));
            world.flush_commands();
        });
        world.commands().queue(add_index(4));

        let _ = catch_unwind(AssertUnwindSafe(|| {
            world.flush_commands();
        }));

        world.commands().queue(add_index(5));
        world.flush_commands();
        assert_eq!(&world.resource::<Order>().0, &[1, 2, 5]);
    }

    #[test]
    fn test_command_queue_inner_nested_panic_safe_handled() {
        #[derive(Resource, Default)]
        struct Order(Vec<usize>);

        fn record_last_error(error: BevyError, context: ErrorContext) {
            *LAST_ERROR.lock().unwrap() = Some((error, context));
        }
        static LAST_ERROR: Mutex<Option<(BevyError, ErrorContext)>> = Mutex::new(None);
        *LAST_ERROR.lock().unwrap() = None;

        let mut world = World::new();
        world.init_resource::<Order>();
        world.insert_resource(FallbackErrorHandler(record_last_error));

        fn add_index(index: usize) -> impl Command {
            move |world: &mut World| world.resource_mut::<Order>().0.push(index)
        }
        world.commands().queue(add_index(1));
        world.commands().queue(|world: &mut World| {
            world.commands().queue(add_index(2));
            world.commands().queue(PanicCommand("I panic!".to_owned()));
            // Everything after here will get run because the
            // fallback error handler handles the panicking command.
            world.commands().queue(add_index(3));
            world.flush_commands();
        });
        world.commands().queue(add_index(4));

        world.flush_commands();

        world.commands().queue(add_index(5));
        world.flush_commands();
        assert_eq!(&world.resource::<Order>().0, &[1, 2, 3, 4, 5]);

        let (error, context) = LAST_ERROR.lock().unwrap().take().unwrap();
        assert!(error.to_string().contains("Command panicked"));
        let name = DebugName::type_name_of_val(&PanicCommand(String::new()).handle_error());
        assert_eq!(context, ErrorContext::Command { name });
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
        type Out = ();

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
