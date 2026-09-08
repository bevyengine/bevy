---
title: "`Escape` in a text input releases focus and propagates"
pull_requests: [25105]
---
Pressing `Escape` while in a focused `EditableText` used to collapse the
selection and consume the event: focus stayed in the field, and no
ancestor or window-level `Escape` handler could run while a text input was
focused. `Escape` now collapses the selection, clears `InputFocus`, and is
no longer consumed: the `FocusedInput<KeyboardInput>` event continues to
bubble to ancestor observers and the window.

If you handle `Escape` on an ancestor of a text input (a dialog's cancel
action, for example) or on the window, that handler now runs on the same
press that blurs the field, where previously it never ran at all. This
matches platform and web behavior, where `Escape` inside a dialog's input
closes the dialog in one press.

If you would rather have two-step behavior (first `Escape` only blurs the
field, the next one reaches your handler), skip presses that originated
inside a text input. Note that `InputFocus` is already cleared by the time
your observer runs, and the event's target field is rewritten at each
propagation hop, so test the trigger's original target instead:

```rust
fn on_escape(
    input: On<FocusedInput<KeyboardInput>>,
    text_inputs: Query<(), With<EditableText>>,
) {
    if text_inputs.contains(input.original_event_target()) {
        // This press blurred a text input; wait for the next one.
        return;
    }
    // cancel / close / navigate back ...
}
```
