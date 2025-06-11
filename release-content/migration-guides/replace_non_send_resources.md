---
title: Replace `Gilrs`, `AccessKitAdapters`, and `WinitWindows` resources
pull_requests: [18386, 17730, 19575]
---

As an effort to remove `!Send` resources in Bevy, we replaced the following resources:
* `Gilrs` - _For wasm32 only, other platforms are unchanged -_ Replaced with `bevy_gilrs::GILRS`
* `WinitWindows` - Replaced with `bevy_winit::WINIT_WINDOWS`
* `AccessKitAdapters` - Replaced with `bevy_winit::ACCESS_KIT_ADAPTERS`

Each of these are now using `thread_local`s to store the data and are temporary solutions to storing `!Send` data. Even though `thread_local`s are thread safe, they should not be accessed from other threads. If they are accessed from other threads, the data will be uninitialized in each non-main thread, which isn't very useful.

Here is an example of how the data can now be accessed. This example will use `WINIT_WINDOWS` as an example, but the same technique can be applied to the others:

__Immutable Access__
```rust
use bevy_winit::WINIT_WINDOWS;

...

WINIT_WINDOWS.with_borrow(|winit_windows| {
    // do things with `winit_windows`
});
```

__Mutable Access__
```rust
use bevy_winit::WINIT_WINDOWS;

...

WINIT_WINDOWS.with_borrow_mut(|winit_windows| {
    // do things with `winit_windows`
});
```

If a borrow is attempted while the data is borrowed elsewhere, the method will panic.
