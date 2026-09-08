---
title: "EditableText readonly and display-only modes"
pull_requests: []
---

Previously, the `EditableText` component functioned as both a holder for the state of a editable
text, and as a standalone widget, with observers and keyboard mappings. These two functions have
been separated: to make a complete, working text input widget, you will now need to insert
both an `EditableText` and a `TextInput` component.
