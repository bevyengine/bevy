---
title: "`Entity` serializes using its `Display` format in human-readable formats"
pull_requests: [25900]
---

`Entity` now serializes as its `Display` string (`{index}v{generation}`, or `PLACEHOLDER`) in human-readable formats like JSON and RON.
Binary formats like `postcard` and MessagePack still use the `u64` from `Entity::to_bits`.

Human-readable formats no longer accept the old `u64` form when deserializing.

```json
// 0.19
{ "entity": 4294967290 }

// 0.20
{ "entity": "5v0" }
```

This affects Bevy Remote Protocol clients that send or parse entity ids,
and hand-written or previously saved `.scn.ron` files, whose entity map keys need to be rewritten from `4294967290: (` to `"5v0": (`.

`Entity` also now implements `FromStr`, which parses the `Display` form and returns a `ParseEntityError` on failure.
