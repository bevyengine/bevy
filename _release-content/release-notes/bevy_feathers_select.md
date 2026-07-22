---
title: FeathersSelect control
authors: ["@gagnus"]
pull_requests: [24847]
---

## Goals

- Adds a new dropdown select control to feathers, for selecting one of a number of options. It is similar to the existing `FeathersListView` (and indeed is implemented using one), it also uses a `FeathersMenuPopup` to show the selection
when opened.
- The dropdown scrolls if there are more than `@max_visible` options
- Adds a helper `caption(...)` function, similar to `label()` which emits `(Text(...) ThemedText)`
- Adds a `SetSelected` event to the underlying `ListBox` bevy_ui_widgets control, which sets its selected value, similar to how `SetChecked` and `SetSliderValue` work.

### Usage

Inside a `bsn!` macro use FeathersSelect scene entity and give it a number of options which contain the `FeathersListRow` component

```rust
(
  @FeathersSelect {
      @options: ... a Box<dyn SceneList> set of rows, see feathers_gallery for an example
      @max_visible: 6,
  }
),
```
