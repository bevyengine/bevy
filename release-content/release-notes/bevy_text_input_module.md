---
title: `bevy_text::input` module
authors: ["@Ickshonpe", "@Zeophlite"]
pull_requests: [20366, 20763]
---

A long-standing feature request from our users is support for text input. Whether the user is creating a new character, logging in with a username and password, or creating a new save file, it's vitally important for them to be able to enter a string of text. Unfortunately, writing a robust and feature-rich text input widget is not easy, especially one that supports all of the expected capabilities (such as range selection, and scrolling). This effort is made much easier now that Bevy has incorporated the `cosmic_text` crate for text handling: much of the underlying logic is handled by them.

Features:

* Placeholder text for empty inputs.
* Password mode.
* Filters applied at edit.
* Autopropagated events emitted on submission, invalid edits and text changes.
* Input method agnostic, users queue `TextEdit`s to make changes to the text input's buffer.
* Max character limit.
* Responsive height sizing.
* Vertical and horizontal scrolling
* Text selection.
* Cut, copy and paste.
* Numeric input mode.
* Multi-line text input with an optional single line mode.
* Support for the common navigation actions like arrow key navigation, home, end, page down, page up, next word, end of paragraph, etc.
* Backspace.
* Overwrite mode.
* Click to place cursor.
* Drag to select.
* Double click to select a word.
* Triple click to select a line.
* Indent and unindent.
* A `TextInputValue` component that contains a copy of the buffer's text and is automatically synchronized on edits. On insertion the `TextInputValue`s contents replace the current text in the `TextInputBuffer`.
* Support for bidirectional text, including RTL scripts such as Arabic and Hebrew.

Check out the `text_input_2d` example to see a usage example.
