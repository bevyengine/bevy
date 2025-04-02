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

- What, then why, then how to migrate.
- Some helpful standardized phrases:
  - `OldType` is now `NewType`. Replace all references and imports.
  - The `Struct::method()` method now requires an additional `magnitude: f32` argument.
  - `Enum` has a new variant, `Enum::NewVariant`, which must be handled during `match` statements.
  - The `Type::method` method has been removed. Use `Type::other_method` instead.
  - The `crate::old_module` module is now `crate::new_module`. Update your imports.
  - `function` now returns `Option<String>`, instead of `String`.
- Make sure it's searchable by directly naming the types and methods involved.
- Use backticks for types, methods and modules (e.g. `Vec<T>` or `core::mem::swap`).
- Use bullet points to explain complex changes.
- Avoid headings. If you must, use only level-two headings.
- Diff codeblocks can be useful for succinctly communicating changes.
  
  ```diff
   fn my_system(world: &mut World) {
  +    world.new_method();
  -    world.old_method();
   }
  ```
  
- Make sure to reference the currently published version of a crate when writing a migration guide.
  See [docs.rs](https://docs.rs/) for a quick reference to the existing public API.
- When moving items to a new module or crate, consider a simple table listing
  the moved items and the before and after paths.
  For example, _`Foo` has been moved from `bar::foo` to `baz`_ could be written:
  
  **Relocations**
  
  | Item                         | Old Path                       | New Path                       |
  | ---------------------------- | ------------------------------ | ------------------------------ |
  | `Foo`                        | `bar::foo`                     | `baz`                          |
