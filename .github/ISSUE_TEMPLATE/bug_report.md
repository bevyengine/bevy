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

These issues are often specific to hardware or OS configuration and can be challenging to reproduce.
As a result, please consider including information about:

- the Rust version you're using (you can get this by running `cargo --version`)
  - Bevy has a policy of relying on the "latest stable release" of Rust
  - Nightly should generally work, but there are sometimes regressions: please let us know!
- the operating system, including its version
  - e.g. Windows 10, Ubuntu 18.04, iOS 14.
- relevant hardware
  - if your bug is rendering-related, copy the adapter info that appears when you run Bevy
  - e.g. `AdapterInfo { name: "NVIDIA GeForce RTX 2070", vendor: 4318, device: 7938, device_type: DiscreteGpu, backend: Vulkan }` or XBox series X controller

You should also consider testing the examples of our upstream dependencies to help isolate the issue:

- [`wgpu`](https://github.com/gfx-rs/wgpu) for rendering problems
- [`winit`](https://github.com/rust-windowing/winit) for input and window management
- [`gilrs`](https://docs.rs/gilrs/latest/gilrs/) for gamepad inputs

## What you did

The steps you took to uncover this bug.
Please provide a runnable code snippet or link to an example that demonstrates the problem if you can.

For example:

```rust
use bevy::prelude::-;

fn main(){
    App::new()
    .add_plugins(DefaultPlugins)
    .run();
}
```

## What went wrong

If it's not clear:

- what were you expecting?
- what actually happened?

## Additional information

Other information that can be used to further reproduce or isolate the problem.
This commonly includes:

- screenshots
- logs
- theories about what might be going wrong
- workarounds that you used
- links to related bugs, PRs or discussions
