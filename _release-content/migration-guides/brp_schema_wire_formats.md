---
title: "`registry.schema` describes the serialized form of opaque types"
pull_requests: [26337]
---

Opaque types have no reflected structure, so `registry.schema` used to give no `type` for most of them.
Opaque types with the new `SerializedAs` annotation now describe their serialized form with standard JSON Schema keywords such as `type`, `prefixItems`, `properties`, `format`, `pattern` and `minimum`.

These types have the annotation:

- `Entity`, `Cow<'static, str>`, `PathBuf`, `SocketAddr` and `AssetPath` are strings.
- `Uuid` and `NonNilUuid` are strings with `format: "uuid"`.
- The `NonZero*` integers are integers that exclude zero.
- `Duration` is an object with `secs` and `nanos`.
- `BVec3A` and `BVec4A` are fixed-length arrays of booleans.

Other opaque types that serialize to JSON of unknown shape now have `rawJson: true`.

To describe your own opaque type, add an annotation such as `#[reflect(opaque, @SerializedAs::STRING)]`.

Rust API changes:

- `JsonSchemaBevyType` has new `raw_json` and `extra` fields. `extra` holds other keywords, such as `format`, `pattern` or `not`.
- `TypeSchema::raw_json` returns whether a type serializes to JSON of unknown shape.
