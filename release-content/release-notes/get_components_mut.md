---
title: get_components_mut
authors: ["@hymm"]
pull_requests: [21780]
---

A safe version of `EntityMut::get_components_mut` and `EntityWorldMut::get_components_mut`
was added. Previously a unsafe version was added `get_components_mut_unchecked`. It needed
to be unsafe because specifying (&mut T, &mut T) is possible which would return multiple
mutable references to the same component. This was done by adding a O(n^2) check for
conflicts which returns a `QueryAccessError::Conflict`. Because of the cost of the checks
if your code is performance sensitive it may make sense to keep using
`get_components_mut_unchecked`.
