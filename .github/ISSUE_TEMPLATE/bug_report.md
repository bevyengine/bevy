---
name: Bug Report
about: Report a bug to help us improve!
title: ''
labels: C-Bug, S-Needs-Triage
assignees: ''
---

## Bevy version

The release number or commit hash of the version you're using.

## \[Optional\] Setup information

If you are reporting a bug about:

- difficulties getting Bevy to build or run on your machine
- unusual rendering bugs
- unusual input bugs
- hardware specific problems

These issues are often specific to hardware or OS configuration, and can be challenging to reproduce.
As a result, please consider including information about:

- the Rust version you're using
  - Bevy has a policy of relying on the "latest stable release" of Rust
  - Nightly should generally work, but there are sometimes regressions: please let us know!
- the operating system, including its version
  - e.g. Windows 10, Ubuntu 18.04, iOS 14.
- relevant hardware
  - e.g. Nvidia GTX 1080 graphics card or XBox series X controller

You should also consider testing the examples of our upstream dependencies to help isolate the issue:

- [`wgpu`](https://github.com/gfx-rs/wgpu) for rendering problems
- [`winit`](https://github.com/rust-windowing/winit) for input and window management
- [`gilrs`](https://docs.rs/gilrs/latest/gilrs/) for gamepad inputs

## What you did

The steps you took to uncover this bug.
Please provide a runnable snippet that demonstrates the problem if feasible.

For example:

```rust
use bevy::prelude::-;

fn main(){
    App::new()
    .add_plugins(DefaultPlugins)
    .add_system(hello_world)
    .run();
}

fn hello_world(){
    println!("Hello World");
}
```

If you can't produce a minimal reproduction, linking to a repository can also be very helpful.

## What went wrong

If it's not immediately obvious:

- what where you expecting?
- what actually occured?

## Additional information

Any additional information you would like to add such as screenshots, logs, etc.
