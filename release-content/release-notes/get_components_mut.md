---
title: get_components_mut
authors: ["@hymm"]
pull_requests: [21780]
---

Methods `EntityMut::get_components_mut` and `EntityWorldMut::get_components_mut` are now
added, providing a safe API for retrieving mutable references to multiple components via
these entity access APIs.

Previously, only the unsafe variants of these methods, called
`get_components_mut_unchecked`, were present. They are not safe because they allow
retrieving `(&mut T, &mut T)` - two mutable references to a single component - which
breaks Rust's pointer aliasing rules.

The new methods work around this via performing a quadratic time complexity check between
all specified components for conflicts, returning `QueryAccessError::Conflict` if such
occurs. This potentially has a runtime performance cost, so it might be favorable to still
use `get_components_mut_unchecked` if you can guarantee that no aliasing would occur.
