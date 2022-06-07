---
name: Bug Report
about: Report a bug to help us improve!
title: ''
labels: C-Bug, S-Needs-Triage
assignees: ''
---

## Bevy version

The release number or commit hash of the version you're using.

## \[Optional\] Relevant system information

If you cannot get Bevy to build or run on your machine, please include:

- the Rust version you're using (you can get this by running `cargo --version`)
  - Bevy relies on the "latest stable release" of Rust
  - nightly should generally work, but there are sometimes regressions: please let us know!
- the operating system or browser used, including its version
  - e.g. Windows 10, Ubuntu 18.04, iOS 14

If your bug is rendering-related, copy the adapter info that appears when you run Bevy.

```ignore
`AdapterInfo { name: "NVIDIA GeForce RTX 2070", vendor: 4318, device: 7938, device_type: DiscreteGpu, backend: Vulkan }`
```

You should also consider testing the examples of our upstream dependencies to help isolate any setup-specific issue:

- [`wgpu`](https://github.com/gfx-rs/wgpu) for rendering problems
- [`winit`](https://github.com/rust-windowing/winit) for input and window management
- [`gilrs`](https://docs.rs/gilrs/latest/gilrs/) for gamepad inputs

## What you did

Describe how you arrived at the problem. If you can, consider providing a code snippet or link.

## What went wrong

If it's not clear, break this out into:

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
