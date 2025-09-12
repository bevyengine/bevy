---
title: Original target of `Pointer` picking events is now stored on observers
pull_requests: [19663]
---

The `Pointer.target` field, which tracks the original target of the pointer event before bubbling, has been removed.
Instead, all "bubbling entity event" observers now track this information, available via the `On::original_entity()` method.

If you were using this information via the Pointer API of picking, please migrate to observers.
If you cannot for performance reasons, please open an issue explaining your exact use case!

As a workaround, you can transform any entity-event into a Message that contains the targeted entity using an observer than writes messages.

```rust
#[derive(Message)]
struct EntityEventMessage<E: EntityEvent> {
    entity: Entity,
    event: E,
}

// A generic observer that handles this transformation
fn transform_entity_event<E: EntityEvent>(event: On<E>, message_writer: MessageWriter<EntityEventMessage<E>>){
    if event.entity() == event.original_entity() {
        message_writer.send(EntityEventMessage {
            event: event.event().clone(),
            entity: event.entity(),
        );
    }
}
```
