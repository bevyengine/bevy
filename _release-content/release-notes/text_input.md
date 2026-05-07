---
title: "Text input"
authors: ["@ickshonpe", "@Zeophlite", "@alice-i-cecile", "@chronicl"]
pull_requests: [19106, 23282, 23455, 23475, 23479, 23496, 23679, 23704, 23841, 23947, 23960, 23969, 24023, 24028, 24032]
---

*TODO: Add a GIF of the EditableText widget with cursor and selection.*

While the ability to capture text is a core requirement for game dev tooling, it's a common task even in games themselves.
Player names, search bars and chat all rely on the ability to enter and submit plain text.

In Bevy 0.19, we've added basic support for text entry, in the form of the `EditableText` widget.
Spawning an entity with this component will create a simple unstyled rectangle of editable text.
Our initial text entry supports:

- Press keys on your keyboard, get text (wow!).
- Cursor navigation: arrow keys, Home/End, and word-level shortcuts (Ctrl/Alt+arrow).
- Selection: Shift+arrow extends by character or word; click and drag with the pointer.
- Multi-click: double-click to select a word, triple-click to select the whole line.
- Backspace and Delete, both for single characters and words.
- Clipboard: uses the OS clipboard with the `system_clipboard` feature enabled, or an in-app buffer without it.
- Unicode-aware navigation and editing: 1 byte/char != 1 character.
- Bidirectional text support, allowing both left-to-right and right-to-left scripts.
- IME (Input Method Editor) support for CJK and other composing scripts.
- Multiline support: newlines, soft-wrapping, and vertical scrolling.
- Horizontal scrolling when content exceeds the visible width.
- Per-character input filtering via `EditableTextFilter`.
- Optional select-all on focus with the `SelectAllOnFocus` component.
- Max character limits via `EditableText::max_characters`.

Many important features are currently unimplemented (placeholder text, undo-redo, password masking...).
While we've been careful to expose and document the internals so that you can readily implement these features in your own projects,
we would like to continue to expand the functionality of the base widget.
Please consider making a PR!

## Usage

To get started, spawn an entity with the `EditableText` component.

```rust
commands.spawn((
    Node {
        width: px(200),
        border: px(2).all(),
        padding: px(8).all(),
        ..default()
    },
    BorderColor::from(Color::WHITE),
    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
    EditableText::default(),
    TextFont {
        font_size: FontSize::Px(24.0),
        ..default()
    },
    TextCursorStyle::default(),
));
```

When working with text input, you'll probably want to add the pre-existing `TabNavigationPlugin` as well, to allow users to easily swap input focus.

To read and clear the input on submission:

```rust
fn on_submit(
    input_focus: Res<InputFocus>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut inputs: Query<&mut EditableText>,
) {
    if keyboard.just_pressed(KeyCode::Enter)
        && let Some(entity) = input_focus.get()
        && let Ok(mut input) = inputs.get_mut(entity)
    {
        println!("Submitted: {}", input.value());
        input.clear();
    }
}
```

`EditableText` integrates with Bevy's `InputFocus` resource, accepting keyboard inputs only when the selected
`EditableText` entity is focused.

The event `TextEditChange` is emitted *after* changes have been applied to the `EditableText`.

## Feathers text input

If you're building editor tooling with Bevy Feathers, there's a pre-built alternative: `FeathersTextInput`.
It wraps `EditableText` and handles several things for you automatically:

- A focus ring appears on the container when the input is focused, and disappears when it isn't.
- Cursor and selection colors update to match the active `UiTheme`.
- The mouse cursor changes to a text beam on hover.
- `TabIndex` is set so keyboard tab navigation works without any extra setup.

You still subscribe to `On<TextEditChange>` to react to the text value — like all Feather's widgets, it handles presentation, not your application logic.

This widget is structured as a container/inner pair:

```rust
bsn! {
    :FeathersTextInputContainer
    Children [
        (
            :FeathersTextInput {
                @max_characters: 20usize,
            }
            MyMarker
            on(on_text_change)
        )
    ]
}

fn on_text_change(
    _trigger: On<TextEditChange>,
    input: Single<&EditableText, With<MyMarker>>,
) {
    println!("{}", input.value());
}
```

Use `EditableText` directly when you need full control over appearance — a way to get the player's name, a styled chat box, or a search bar in your game's UI.
Use `FeathersTextInput` when you want a polished, Feathers-themed widget out of the box.
