---
title: "`registry.schema` matches serialized BRP values"
pull_requests: [26308]
---

`registry.schema` now describes types the way BRP serializes them, so serialized values validate against it with a standard JSON Schema 2020-12 validator.

- Tuple structs with exactly one field (and no skipped fields) serialize as their bare inner value.
  Their schema keeps `kind: "TupleStruct"`, but has a `$ref` to the inner type instead of `type: "array"`, `prefixItems` and `items`.
- Fields marked `#[reflect(skip_serializing)]` are no longer listed in `properties`, `required` or `prefixItems`.
- `Option<T>` serializes as the inner value or `null`.
  Its schema has no top-level `type`, its `None` entry in `oneOf` has `type: "null"`, and its `Some` entry has a `$ref` to `T` and `not: {"type": "null"}`.
- Enum variants with data serialize as `{"Variant": value}`.
  Each such entry in `oneOf` is now an object schema with a single required `Variant` property.
  The variant's fields moved into that property: struct variant fields are under `properties.<Variant>.properties`, tuple variant items under `properties.<Variant>.prefixItems`, and a single-field tuple variant is a `$ref` to the field type.
  `kind`, `typePath`, `shortPath` and `annotations` stay on the entry.
- Unit variants serialize as their name. Their `oneOf` entry is now `{"const": "Variant", "typePath": ..., "shortPath": ...}`, including in all-unit enums, where entries used to be plain strings.
- Enums with any data variant no longer have a top-level `type`. All-unit enums keep `type: "string"`.
- `type` only uses standard JSON Schema types.
  `float` is now `number`, `int` is now `integer`, and `uint` is now `integer` with `minimum: 0`.
  Sets have `type: "array"` and `uniqueItems: true`; `kind: "Set"` still identifies them.
  Opaque types other than primitives and strings no longer claim `type: "object"` and have no `type`.

Maps serialize as JSON objects. Their schema now has `additionalProperties` with a `$ref` to the value type, next to the existing `keyType` and `valueType`.

Annotated types, fields and variants now also carry the standard `title`, `description`, `minimum`, `maximum` and `readOnly` keywords, next to the existing `annotations` object.

Rust API changes:

- `JsonSchemaBevyType::schema_type` is now an `Option<SchemaType>`.
- `JsonSchemaBevyType::additional_properties` is now an `Option<Value>`, holding either `false` or a schema.
- `JsonSchemaBevyType` has new `reference` (`$ref`), `title`, `description`, `minimum`, `maximum`, `read_only` and `unique_items` fields.
- `SchemaType::Float` is replaced by `SchemaType::Number`, and `SchemaType::Int` and `SchemaType::Uint` by `SchemaType::Integer`. `SchemaType::Set` was removed; use `SchemaType::Array`.
- `SchemaJsonType::map_json_type` now returns `Option<SchemaType>`, with `None` for types whose JSON type is not known.
