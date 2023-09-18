---
name: Performance Regression
about: Bevy running slowly after upgrading? Report a performance regression.
title: ''
labels: C-Bug, C-Performance, C-Regression, S-Needs-Triage
assignees: ''
---

## Bevy version

* Original: The release number or commit hash of the version you last tested your app against.
* Current: The release number or commit hash of the version you're currently using.

## Relevant system information

Please include:

* the Rust version you're using (you can get this by running `cargo --version`)
  * Bevy relies on the "latest stable release" of Rust
  * nightly should generally work, but there are sometimes regressions: please let us know!
* the operating system or browser used, including its version
  * e.g. Windows 10, Ubuntu 18.04, iOS 14

## What's performing poorly?

Describe how you arrived at the problem. If you can, consider providing a code snippet or link
to help reproduce the regression.

If the exact scenario is not immediately reproducible on `cargo run`, please include a set list of steps to produce the correct setup.

## Before and After Traces

To best help us investigate the regression, it's best to provide as much detailed profiling
data as possible.

If your app is running slowly, please show profiler traces before and after the change.
For more information on how to get these traces, see
<https://github.com/bevyengine/bevy/blob/main/docs/profiling.md>.

If this is about a compile-time regression, please provide the full output of `cargo build --timings`,
for more information see <https://doc.rust-lang.org/cargo/reference/timings.html>.

* Before:
* After:

## Additional information

Other information that can be used to further reproduce or isolate the problem.
This commonly includes:

* screenshots
* logs
* theories about what might be going wrong
* workarounds that you used
* links to related bugs, PRs or discussions
