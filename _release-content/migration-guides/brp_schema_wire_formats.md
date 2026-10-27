---
title: "`registry.schema` describes custom serialized forms"
pull_requests: []
---

BRP serializes types that register `ReflectSerialize` with their `Serialize` implementation, which does not always follow their reflected structure.
`registry.schema` now describes the serialized form for such types.

- Types that register the new `ReflectBrpJsonSchema` type data use the keywords it declares at the top level.
  The reflected `type`, `$ref`, `properties`, `required`, `additionalProperties`, `prefixItems`, `items`, `minItems`, `maxItems`, `oneOf` and `uniqueItems` move into a new `reflectSchema` object.
  `kind` still describes the reflected structure.
- `RemotePlugin` registers it for `glam` vectors, quaternions, matrices and affines (fixed-length arrays whose items are titled `x`, `x_axis.y`, `translation.z`, ...), `Entity`, `Name`, `Cow<'static, str>`, `PathBuf`, `SocketAddr`, `Duration`, the `NonZero*` integers, and, with the `bevy_asset` feature, `AssetPath`, `Uuid` and `NonNilUuid`.
  For example, `glam::Vec3` now has `type: "array"` with three `prefixItems`, and its `x`, `y` and `z` properties are under `reflectSchema`.
- Other opaque types that serialize to JSON of unknown shape now have `rawJson: true`.

To describe your own type, implement `BrpJsonSchema` and add `#[reflect(BrpJsonSchema)]`.

Rust API changes:

- `JsonSchemaBevyType` has new `min_items`, `max_items`, `raw_json`, `reflect_schema` and `extra` fields. `extra` holds other keywords, such as `format`, `pattern` or `not`.
- `SchemaRegistry` and `TypeSchema` still read the reflected structure. Use `TypeSchema::wire` for the serialized form and `TypeSchema::raw_json` to detect unknown shapes.
