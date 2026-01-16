//! From time to time, you may find that you want to both send and receive a message of the same type in a single system.
//!
//! Of course, this results in an error: the borrows of [`MessageWriter`] and [`MessageReader`] overlap,
//! if and only if the [`Message`] type is the same.
//! One system parameter borrows the [`Messages`] resource mutably, and another system parameter borrows the [`Messages`] resource immutably.
//! If Bevy allowed this, this would violate Rust's rules against aliased mutability.
//! In other words, this would be Undefined Behavior (UB)!
//!
//! There are two ways to solve this problem:
//!
//! 1. Use [`ParamSet`] to check out the [`MessageWriter`] and [`MessageReader`] one at a time.
//! 2. Use a [`Local`] [`MessageCursor`] instead of a [`MessageReader`], and use [`ResMut`] to access [`Messages`].
//!
//! In the first case, you're being careful to only check out only one of the [`MessageWriter`] or [`MessageReader`] at a time.
//! By "temporally" separating them, you avoid the overlap.
//!
//! In the second case, you only ever have one access to the underlying  [`Messages`] resource at a time.
//! But in exchange, you have to manually keep track of which messages you've already read.
//!
//! Let's look at an example of each.

use bevy::{diagnostic::FrameCount, ecs::message::MessageCursor, prelude::*};

fn main() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_message::<DebugMessage>()
        .add_message::<A>()
        .add_message::<B>()
        .add_systems(Update, read_and_write_different_message_types)
        .add_systems(
            Update,
            (
                send_messages,
                debug_messages,
                send_and_receive_param_set,
                debug_messages,
                send_and_receive_manual_message_reader,
                debug_messages,
            )
                .chain(),
        );
    // We're just going to run a few frames, so we can see and understand the output.
    app.update();
    // By running for longer than one frame, we can see that we're caching our cursor in the message queue properly.
    app.update();
}

#[derive(Message)]
struct A;

#[derive(Message)]
struct B;

// This works fine, because the types are different,
// so the borrows of the `MessageWriter` and `MessageReader` don't overlap.
// Note that these borrowing rules are checked at system initialization time,
// not at compile time, as Bevy uses internal unsafe code to split the `World` into disjoint pieces.
fn read_and_write_different_message_types(mut a: MessageWriter<A>, mut b: MessageReader<B>) {
    for _ in b.read() {}
    a.write(A);
}

/// A dummy message type.
#[derive(Debug, Clone, Message)]
struct DebugMessage {
    resend_from_param_set: bool,
    resend_from_local_message_reader: bool,
    times_sent: u8,
}

/// A system that sends all combinations of messages.
fn send_messages(mut debug_messages: MessageWriter<DebugMessage>, frame_count: Res<FrameCount>) {
    println!("Sending messages for frame {}", frame_count.0);

    debug_messages.write(DebugMessage {
        resend_from_param_set: false,
        resend_from_local_message_reader: false,
        times_sent: 1,
    });
    debug_messages.write(DebugMessage {
        resend_from_param_set: true,
        resend_from_local_message_reader: false,
        times_sent: 1,
    });
    debug_messages.write(DebugMessage {
        resend_from_param_set: false,
        resend_from_local_message_reader: true,
        times_sent: 1,
    });
    debug_messages.write(DebugMessage {
        resend_from_param_set: true,
        resend_from_local_message_reader: true,
        times_sent: 1,
    });
}

/// A system that prints all messages sent since the last time this system ran.
///
/// Note that some messages will be printed twice, because they were sent twice.
fn debug_messages(mut messages: MessageReader<DebugMessage>) {
    for message in messages.read() {
        println!("{message:?}");
    }
}

/// A system that both sends and receives messages using [`ParamSet`].
fn send_and_receive_param_set(
    mut param_set: ParamSet<(MessageReader<DebugMessage>, MessageWriter<DebugMessage>)>,
    frame_count: Res<FrameCount>,
) {
    println!(
        "Sending and receiving messages for frame {} with a `ParamSet`",
        frame_count.0
    );

    // We must collect the messages to resend, because we can't access the writer while we're iterating over the reader.
    let mut messages_to_resend = Vec::new();

    // This is p0, as the first parameter in the `ParamSet` is the reader.
    for message in param_set.p0().read() {
        if message.resend_from_param_set {
            messages_to_resend.push(message.clone());
        }
    }

    // This is p1, as the second parameter in the `ParamSet` is the writer.
    for mut message in messages_to_resend {
        message.times_sent += 1;
        param_set.p1().write(message);
    }
}

/// A system that both sends and receives messages using a [`Local`] [`MessageCursor`].
fn send_and_receive_manual_message_reader(
    // The `Local` `SystemParam` stores state inside the system itself, rather than in the world.
    // `MessageCursor<T>` is the internal state of `MessageReader<T>`, which tracks which messages have been seen.
    mut local_message_reader: Local<MessageCursor<DebugMessage>>,
    // We can access the `Messages` resource mutably, allowing us to both read and write its contents.
    mut messages: ResMut<Messages<DebugMessage>>,
    frame_count: Res<FrameCount>,
) {
    println!(
        "Sending and receiving messages for frame {} with a `Local<MessageCursor>",
        frame_count.0
    );

    // We must collect the messages to resend, because we can't mutate messages while we're iterating over the messages.
    let mut messages_to_resend = Vec::new();

    for message in local_message_reader.read(&messages) {
        if message.resend_from_local_message_reader {
            // For simplicity, we're cloning the message.
            // In this case, since we have mutable access to the `Messages` resource,
            // we could also just mutate the message in-place,
            // or drain the message queue into our `messages_to_resend` vector.
            messages_to_resend.push(message.clone());
        }
    }

    for mut message in messages_to_resend {
        message.times_sent += 1;
        messages.write(message);
    }
}
