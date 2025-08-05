---
title: Event Split
authors: ["@Jondolf"]
pull_requests: [19647, 20101]
---

In past releases, all event types were defined by simply deriving the `Event` trait:

```rust
#[derive(Event)]
struct Speak {
    message: String,
}
```

You could then use the various event handling tools in Bevy to send and listen to the event. The common options include:

- Use `trigger` to trigger the event and react to it with a global `Observer`
- Use `trigger_targets` to trigger the event with specific entity target(s) and react to it with an entity `Observer` or global `Observer`
- Use `EventWriter::write` to write the event to an event buffer and `EventReader::read` to read it at a later time

The first two are observer APIs, while the third is a fully separate "buffered" API for pull-based event handling.
All three patterns are fundamentally different in both the interface and usage. Despite the same event type being used everywhere,
APIs are typically built to support only one of them.

This has led to a lot of confusion and frustration for users. A common footgun was using a "buffered event" with an observer,
or an observer event with `EventReader`, leaving the user wondering why the event is not being detected.

**Bevy 0.17** aims to solve this ambiguity by splitting the event traits into `Event`, `EntityEvent`, and `BufferedEvent`.

- `Event`: A shared trait for observer events.
- `EntityEvent`: An `Event` that additionally supports targeting specific entities and propagating the event from one entity to another.
- `BufferedEvent`: An event that supports usage with `EventReader` and `EventWriter` for pull-based event handling.

## Using Events

A basic `Event` can be defined like before, by deriving the `Event` trait.

```rust
#[derive(Event)]
struct Speak {
    message: String,
}
```

You can then `trigger` the event, and use a global observer for reacting to it.

```rust
app.add_observer(|trigger: On<Speak>| {
    println!("{}", trigger.message);
});

// ...

commands.trigger(Speak {
    message: "Hello!".to_string(),
});
```

To allow an event to be targeted at entities and even propagated further, you can instead derive `EntityEvent`.
It supports optionally specifying some options for propagation using the `event` attribute:

```rust
// When the `Damage` event is triggered on an entity, bubble the event up to ancestors.
#[derive(EntityEvent)]
#[entity_event(traversal = &'static ChildOf, auto_propagate)]
struct Damage {
    amount: f32,
}
```

Every `EntityEvent` is also an `Event`, so you can still use `trigger` to trigger them globally.
However, entity events also support targeted observer APIs such as `trigger_targets` and `observe`:

```rust
// Spawn an enemy entity.
let enemy = commands.spawn((Enemy, Health(100.0))).id();

// Spawn some armor as a child of the enemy entity.
// When the armor takes damage, it will bubble the event up to the enemy,
// which can then handle the event with its own observer.
let armor_piece = commands
    .spawn((ArmorPiece, Health(25.0), ChildOf(enemy)))
    .observe(|trigger: On<Damage>, mut query: Query<&mut Health>| {
        // Note: `On::target` only exists because this is an `EntityEvent`.
        let mut health = query.get(trigger.target()).unwrap();
        health.0 -= trigger.amount();
    })
    .id();

// Trigger the `Damage` event on the armor piece.
commands.trigger_targets(Damage { amount: 10.0 }, armor_piece);
```

To allow an event to be used with the buffered API, you can instead derive `BufferedEvent`:

```rust
#[derive(BufferedEvent)]
struct Message(String);
```

The event can then be used with `EventReader`/`EventWriter`:

```rust
fn write_hello(mut writer: EventWriter<Message>) {
    writer.write(Message("I hope these examples are alright".to_string()));
}

fn read_messages(mut reader: EventReader<Message>) {
    // Process all buffered events of type `Message`.
    for Message(message) in reader.read() {
        println!("{message}");
    }
}
```

In summary:

- Need a basic event you can trigger and observe? Derive `Event`!
- Need the observer event to be targeted at an entity? Derive `EntityEvent`!
- Need the event to be buffered and support the `EventReader`/`EventWriter` API? Derive `BufferedEvent`!
