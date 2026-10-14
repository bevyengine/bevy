---
title: "`registry.schema` properties are sorted and indexed"
pull_requests: [26162]
---

`JsonSchemaBevyType::properties` is now a `BTreeMap<String, Value>` instead of a `HashMap`, so properties are sorted by name.
Each property in `registry.schema` output now has a `fieldIndex` key with the field's declaration order.

The `typePath` key on properties was removed. Use the property's `$ref` for the field type, and the property's key for the field name.
