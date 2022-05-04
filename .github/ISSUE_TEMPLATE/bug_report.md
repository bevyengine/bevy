---
name: Bug Report
about: Report a bug to help us improve!
title: ''
labels: C-Bug, S-Needs-Triage
assignees: ''
---

## Bevy version

The release number or commit hash of the version you're using.

## Operating system & version

Ex: Windows 10, Ubuntu 18.04, iOS 14.

## What you did

The steps you took to uncover this bug.
Please provide a runnable snippet that demonstrates the problem if feasible.

For example:

```rust
use bevy::prelude::*;

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
