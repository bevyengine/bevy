---
title: Infinite Children
authors: ["@CorvusPrudens"]
pull_requests: [18865]
---

The `children!` macro is a convenient way to spawn children alongside their parents in Bevy code.
When it was introduced in **Bevy 0.16** this was limited to 12 children, due to arbitrary limitations (Rust: please [support variadic generics!](https://blog.rust-lang.org/inside-rust/2025/09/11/program-management-update-2025-08/#variadic-generics)), and not implementing the requisite workarounds.
When working with large UI hierarchies, this could be a real nuisance, forcing users to resort to ugly workarounds.

We've rewritten the macro and lifted this unjust restriction. You are now only limited by Rust's recursion limit: around 1400 children at once.
Rejoice!
If you are manually spawning more than 1400 children in a single macro call, you should reconsider your strategy (such as using `SpawnIter` or `SpawnWith`).

We've made the same change to the `related!` macro, allowing you to spawn huge numbers of related entities in a single call.
