# Bevy Ptr

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_ptr.svg)](https://crates.io/crates/bevy_ptr)
[![Downloads](https://img.shields.io/crates/d/bevy_ptr.svg)](https://crates.io/crates/bevy_ptr)
[![Docs](https://docs.rs/bevy_ptr/badge.svg)](https://docs.rs/bevy_ptr/latest/bevy_ptr/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

The `bevy_ptr` crate provides low-level abstractions for working with pointers in a more safe way than using rust's raw pointers.

Rust has lifetimed and typed references (`&'a T`), unlifetimed and typed references (`*const T`), but no lifetimed but untyped references.
`bevy_ptr` adds them, called `Ptr<'a>`, `PtrMut<'a>` and `OwningPtr<'a>`.
These types are lifetime-checked so can never lead to problems like use-after-frees and must always point to valid data.
