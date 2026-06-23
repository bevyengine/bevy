---
title: "Add scrubbing / dragging to number_input widget"
pull_requests: [24636, 24701]
---

The API for the `FeathersNumberInput` has changed. To programmatically update the value, instead
of triggering an `UpdateNumberInput` event, you should insert a `NumberInputValue` component.
This makes it easier to specify the initial value at creation.

```wgsl
// BEFORE
commands.trigger(UpdateNumberInput {
    entity: input_ent,
    value: NumberInputValue::F32(new_value),
});

// AFTER
commands
    .entity(input_ent)
    .insert(NumberInputValue::F32(new_value));
```
