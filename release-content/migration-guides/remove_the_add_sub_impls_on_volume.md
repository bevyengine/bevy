---
title: remove the Add/Sub impls on Volume
pull_requests: [ 19423 ]
---

Linear volumes are like percentages, and it does not make sense to add or subtract percentages.
As such, use the new `increase_by_percentage` function instead of addition or subtraction.

```rust
// 0.16
fn audio_system() {
    let linear_a = Volume::Linear(0.5);
    let linear_b = Volume::Linear(0.1);
    let linear_c = linear_a + linear_b;
    let linear_d = linear_a - linear_b;
}

// 0.17
fn audio_system() {
    let linear_a = Volume::Linear(0.5);
    let linear_b = Volume::Linear(0.1);
    let linear_c = linear_a.increase_by_percentage(10.0);
    let linear_d = linear_a.increase_by_percentage(-10.0);
}
```
