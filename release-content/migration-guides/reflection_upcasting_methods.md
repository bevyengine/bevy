---
title: "Deprecated `Reflect` methods: `into_reflect`, `as_reflect`, and `as_reflect_mut`" 
pull_requests: [21772]
---

The `into_any`, `as_any`, `as_any_mut`, `into_reflect`, `as_reflect`, and `as_reflect_mut` methods on the `Reflect` trait have been deprecated,
as [trait upcasting was stabilized](https://github.com/rust-lang/rust/issues/65991) in [Rust 1.86](https://doc.rust-lang.org/beta/releases.html#language-3).

In many cases, these method calls can simply be removed, and the compiler will infer what you meant to do.

In some cases however, it may need a bit of help. Type annotations (e.g. `let foo: Box<dyn Reflect>`) can be quite useful,
and if you are trying to pass in a reference to a method, judicious use of `&*` may be required to resolve compiler errors.
