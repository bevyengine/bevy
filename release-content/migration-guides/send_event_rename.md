---
title: Rename `send_event` and similar methods to `write_event`
pull_requests: [20017]
---

Following up on the `EventWriter::send` being renamed to `EventWriter::write` in 0.16, many similar methods have been renamed.
This includes both the `World` and `Commands` event methods. The old methods have been depreciated.

| Old                                 | New                                  |
|-------------------------------------|--------------------------------------|
| `World::send_event`                 | `World::write_event`                 |
| `World::send_event_default`         | `World::write_event_default`         |
| `World::send_event_batch`           | `World::write_event_batch`           |
| `DeferredWorld::send_event`         | `DeferredWorld::write_event`         |
| `DeferredWorld::send_event_default` | `DeferredWorld::write_event_default` |
| `DeferredWorld::send_event_batch`   | `DeferredWorld::write_event_batch`   |
| `Commands::send_event`              | `Commands::write_event`              |
| `Events::send`                      | `Events::write`                      |
| `Events::send_default`              | `Events::write_default`              |
| `Events::send_batch`                | `Events::write_batch`                |
| `RemovedComponentEvents::send`      | `RemovedComponentEvents::write`      |
| `command::send_event`               | `command::write_event`               |
| `SendBatchIds`                      | `WriteBatchIds`                      |
