---
title: get_components, get_components_mut_unchecked now return a Result
pull_requests: [14791, 15458, 15269]
---

`get_components`, `get_components_mut_unchecked` now return a `Result<_, QueryAccessError>` instead of an `Option`.