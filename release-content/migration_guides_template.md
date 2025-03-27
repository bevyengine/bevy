---
title: Feature that broke
pull_requests: [14791, 15458, 15269]
---

Copy the contents of this file into a new file in `./migration-guides`, update the metadata, and add migration guide content here.

## Goals

Aim to communicate:

- What has changed since the last release?
- Why did we make this breaking change?
- How can users migrate their existing code?

## Style Guide

Keep it short and sweet:

- Use bullet points and make sure it's searchable.
- Avoid headings. If you must, use only level-two headings.
- Use backticks for types (e.g. `Vec<T>`) in either the title or the body.
- Diff codeblocks can also be useful for succinctly communicating changes.

```diff
fn my_system(world: &mut World) {
+ world.new_method();
- world.old_method();
}
```
