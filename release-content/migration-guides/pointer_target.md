---
title: Original target of `Pointer` picking events is now stored on observers
pull_requests: [19663]
---

The `Pointer.target` field, which tracks the original target of the pointer event before bubbling, has been removed.
Instead, all observers now track this information, available via the `On::original_target()` method.

If you were using this information via the buffered event API of picking, please migrate to observers.
If you cannot for performance reasons, please open an issue explaining your exact use case!

As a workaround, you can transform any entity-event into a buffered event that contains the targeted entity using
an observer than emits events.

```rust
#[derive(BufferedEvent)]
struct TransformedEntityEvent<E: EntityEvent> {
    entity: Entity,
    event: E,
}

// A generic observer that handles this transformation
fn transform_entity_event<E: EntityEvent>(trigger: On<E>, event_writer: EventWriter<TransformedEntityEvent<E>>){
    if trigger.target() == trigger.original_target(){
        event_writer.send(trigger.event())
    }
}
```

Additionally, the `ObserverTrigger::target` field has been renamed to `ObserverTrigger::current_target` and a new `ObserverTrigger::original_target` field has been added.
