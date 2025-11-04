---
title: ECS reborrowing traits
pull_requests: [todo]
---

Bevy 0.18 adds a new `ReborrowQueryData` trait to the `QueryData` family, which allows for
shortening the lifetime of a borrowed query item. While not a breaking change, it's recommended
to implement for any custom `QueryData` types
