---
title: Rename `send_event` and similar methods to `write_message`
pull_requests: [20017, 20953]
---

Following up on the `EventWriter::send` being renamed to `EventWriter::write` in 0.16, many similar methods have been renamed. Note that "buffered events" are now known as `Messages`, and the naming reflects that here.

This includes both the `World` and `Commands` message methods. The old methods have been depreciated.

| Old                                 | New                                    |
| ----------------------------------- | -------------------------------------- |
| `World::send_event`                 | `World::write_message`                 |
| `World::send_event_default`         | `World::write_message_default`         |
| `World::send_event_batch`           | `World::write_message_batch`           |
| `DeferredWorld::send_event`         | `DeferredWorld::write_message`         |
| `DeferredWorld::send_event_default` | `DeferredWorld::write_message_default` |
| `DeferredWorld::send_event_batch`   | `DeferredWorld::write_message_batch`   |
| `Commands::send_event`              | `Commands::write_message`              |
| `Events::send`                      | `Messages::write`                      |
| `Events::send_default`              | `Messages::write_default`              |
| `Events::send_batch`                | `Messages::write_batch`                |
| `RemovedComponentEvents::send`      | `RemovedComponentEvents::write`        |
| `command::send_event`               | `command::write_message`               |
| `SendBatchIds`                      | `WriteBatchIds`                        |
