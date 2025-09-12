---
title: "`RemovedComponents` methods renamed to match `Event` to `Message` rename"
pull_requests: [ 20953, 20954 ]
---

As part of the broader shift to differentiate between buffered events (0.16's `EventWriter`/`EventReader`) and observer-events,
various methods and types related to `RemovedComponents` have been renamed.

The implementation of `RemovedComponents` uses buffered events (now, messages):
and as a result, the following types and messages have been renamed:

| Old                                         | New                                           |
| ------------------------------------------- | --------------------------------------------- |
| `RemovedComponents::events`                 | `RemovedComponents::messages`                 |
| `RemovedComponents::reader_mut_with_events` | `RemovedComponents::reader_mut_with_messages` |
| `RemovedComponentEvents`                    | `RemovedComponentMessages`                    |
