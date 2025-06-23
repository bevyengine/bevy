---
title: Event Split
pull_requests: [19647]
---

The `Event` trait was previously used for all types of events: "observer events" with and without targets,
and "buffered events" using `EventReader` and `EventWriter`.

Buffered events and targeted events have now been split into dedicated `BufferedEvent` and `EntityEvent` traits.
An event with just the `Event` trait implemented only supports non-targeted APIs such as global observers and the `trigger` method.

If an event is used with `trigger_targets` or an entity observer, make sure you have derived `EntityEvent` for it.

If an event is used with `EventReader` or `EventWriter`, make sure you have derived `BufferedEvent` for it.
