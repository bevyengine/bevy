---
title: "Text input"
authors: ["@ickshonpe", "@Zeophlite", "@alice-i-cecile"]
pull_requests: [23282, 23455, 23475, 23479]
---

Entering text into an application is a common task, even for games.
Player names, search bars and chat all rely on the ability to enter and submit plain text.

In Bevy 0.19, we've added basic support for text entry, in the form of the `EditableText` widget.
Spawning an entity with this component will create a simple unstyled rectangle of editable text.
Our initial text entry supports:

- Press keys on your keyboard, get text (wow!).
- Navigation using the arrow keys and standard keyboard shortcuts.
- Selection rectangles (hold shift).
- Backspace and Delete, both for single characters and words.
- Pointer support, click to place the cursor and drag to extend selection.
- Unicode-aware navigation and editing: 1 byte/char != 1 character.
- Bidirectional text support, allowing both left-to-right and right-to-left scripts.
- Placeholder clipboard implementation using a `Clipboard` resource. It can't access the OS clipboard, but allows local copy, cut and paste actions to be used inside a bevy app.

`EditableText` integrates with Bevy's `InputFocus` resource, accepting keyboard inputs only when the selected
`EditableText` entity is focused.

The event `TextEditChange` is emitted *after* changes have been applied to the `EditableText`.

Many important features are currently unimplemented (placeholder text, clipboard support, undo-redo...).
While we've been careful to expose and document the internals so that you can readily implement these features in your own projects,
we would like to continue to expand the functionality of the base widget: please consider making a PR!
