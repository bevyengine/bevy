---
title: Infinite Children
authors: ["@CorvusPrudens"]
pull_requests: [18865]
---

The `children!` macro is a convenient way to define a parent-child relationship in Bevy code.
When it was introduced in Bevy 0.16, this was limited to 12 children, due to concerns about compile time due to the initial macro implementation strategy.
This was not, in fact, enough for anyone.
When working with large UI hierarchies, this could be a real nuisance, forcing users to resort to ugly workarounds.

We've rewritten the macro and lifted this unjust restriction. You are now only limited by Rust's recursion limit: around 1400 children at once.
Rejoice!
If you are spawning more than 1400 children in a single macro call, you should probably reconsider your strategy.

We've made the same change to the `related!` macro, allowing you to spawn huge numbers of related entities in a single call.
