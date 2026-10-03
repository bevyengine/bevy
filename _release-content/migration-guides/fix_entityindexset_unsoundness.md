---
title: EntityEquivalentIndexSet no longer implement DerefMut to IndexSet
pull_requests: [25940]
---

For the purpose of fixing a soundness issue `EntityEquivalentIndexSet` can no longer implement `DerefMut` to `IndexSet` or safely yield mutable references to it. It now directly implements most mutable methods of `IndexSet`, and a new `EntityEquivalentIndexSet::as_index_set_unchecked` unsafe method was added to get a `&mut IndexSet` if necessary.

`EntityEquivalentIndexSet::from_index_set` was also renamed to `from_index_set_unchecked` and made unsafe, with the precondition that the set does not contain duplicates.
