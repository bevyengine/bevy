---
title: "`Event` trait split / Rename"
pull_requests: [19647]
---

"Buffered events" (things sent/read using `EventWriter` / `EventReader`) are now no longer referred to as "events", in the interest of conceptual clarity and learn-ability (see the release notes for rationale). "Event" as a concept (and the `Event` trait) are now used solely for "observable events". "Buffered events" are now known as "messages" and use the `Message` trait. `EventWriter`, `EventReader`, and `Events<E>`, are now known as `MessageWriter`, `MessageReader`, and `Messages<M>`. Types can be _both_ "messages" and "events" by deriving both `Message` and `Event`, but we expect most types to only be used in one context or the other.
