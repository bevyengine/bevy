---
title: "`UniqueEntityEquivalentVec` no longer implements `Extend`"
pull_requests: []
---

The `Extend` impl for `UniqueEntityEquivalentVec`has been removed
because it performed an O(N^2) check for duplicates.
This could easily cause performance problems for large collections,
but would be hidden behind an innocent-looking `extend()` call.

The old behavior is available at `UniqueEntityEquivalentVec::extend_slow` if needed,
although in most cases it will be better to replace the `UniqueEntityEquivalentVec` with an `EntityEquivalentIndexSet`.
