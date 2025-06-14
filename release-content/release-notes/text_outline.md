---
title: UI Gradients 
authors: ["@totalkrill", "@UkoeHB"]
pull_requests: [19639]
---

Support for text outline through adding a `TextOutline` component to an entity with a `Text` component

```rust
    commands.spawn((
        // Accepts a `String` or any type that converts into a `String`, such as `&str`
        Text::new("hello\nbevy!"),
        TextOutline {
            width: 1.,
            color: GREEN.into(),
            anti_aliasing: None,
        }));
```

