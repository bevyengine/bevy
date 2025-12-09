---
title: get_components, get_components_mut_unchecked now return a Result
pull_requests: [21780]
---

`get_components`, `get_components_mut_unchecked`, and `into_components_mut_unchecked`
now return a `Result<_, QueryAccessError>` instead of an `Option`.
