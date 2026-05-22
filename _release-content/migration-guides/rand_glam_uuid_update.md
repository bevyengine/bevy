---
title: rand, glam & uuid updated to latest versions
pull_requests: [22928]
---

For `rand`/`rand_core`, the `RngCore` trait is now `Rng`. The `Rng` trait is now `RngExt`, update imports as needed. For the full extent of the changes to `rand` v0.10, consult the [`rand` book here](https://rust-random.github.io/book/update-0.10.html).
