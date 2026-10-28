---
title: Selecting entities by name path over BRP
authors: ["@viridia"]
pull_requests: [26250]
---

The Bevy Remote Protocol's `world.query` can now find entities by a path of names, through a new `name_path` filter.
This is meant for tools that drive a running app from outside, such as end-to-end tests: a test can name the button it wants to click by where it sits in the hierarchy, without knowing its `Entity` id and without fetching the whole hierarchy to search it.

```json
{
    "method": "world.query",
    "params": {
        "data": {},
        "filter": { "name_path": ["Settings", "Jump", "Rebind"] }
    }
}
```

This finds every entity named `Rebind` that has an ancestor named `Jump`, which in turn has an ancestor named `Settings`.
Ancestors can be at any depth, so unnamed entities in between are skipped, and the first name need not be a root.

A number in the path selects a child by its position: `["List", 2, "Label"]` is an entity named `Label` inside the third child of `List`.
This reaches entities that have no name of their own, such as one row of a list built from data.

`name_path` combines with `with` and `without`, and returns every entity that matches.
