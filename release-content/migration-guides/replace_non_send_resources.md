---
title: Replace `Gilrs`, `AccessKitAdapters`, and `WinitWindows` resources
pull_requests: [18386, 17730, 19575]
---

## NonSend Resources Replaced

As an effort to remove `!Send` resources in Bevy, we replaced the following resources:

* `Gilrs` - _For wasm32 only, other platforms are unchanged -_ Replaced with `bevy_gilrs::GILRS`
* `WinitWindows` - Replaced with `bevy_winit::WINIT_WINDOWS`
* `AccessKitAdapters` - Replaced with `bevy_winit::ACCESS_KIT_ADAPTERS`

Each of these are now using `thread_local`s to store the data and are temporary solutions to storing `!Send` data. Even though `thread_local`s are thread safe, they should not be accessed from other threads. If they are accessed from other threads, the data will be uninitialized in each non-main thread, which isn't very useful.

Here is an example of how the data can now be accessed. This example will use `WINIT_WINDOWS` as an example, but the same technique can be applied to the others:

### Immutable Access

```rust
use bevy_winit::WINIT_WINDOWS;

...

WINIT_WINDOWS.with_borrow(|winit_windows| {
    // do things with `winit_windows`
});
```

### Mutable Access

```rust
use bevy_winit::WINIT_WINDOWS;

...

WINIT_WINDOWS.with_borrow_mut(|winit_windows| {
    // do things with `winit_windows`
});
```

If a borrow is attempted while the data is borrowed elsewhere, the method will panic.

## NonSend Systems

Previously, the use of a `!Send` resource in a system would force the system to execute on the main thread. Since `!Send` resources are removed in Bevy, we needed to create a new way to prevent systems from running on non-main threads. To do this, you can now use `bevy_ecs::system::NonSendMarker` as a system parameter:

```rust
use bevy_ecs::system::NonSendMarker;

fn my_system(
    _non_send_marker: NonSendMarker,
) {
    ACCESS_KIT_ADAPTERS.with_borrow_mut(|adapters| {
        // do things with adapters
    });
}
```

To prevent a panic, if any of the `!Send` resource replacements mentioned in this document are used in a system, the system should _always_ be marked as `!Send` with `bevy_ecs::system::NonSendMarker`.
