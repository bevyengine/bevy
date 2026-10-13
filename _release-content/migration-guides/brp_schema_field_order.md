---
title: "`registry.schema` properties are sorted and indexed"
pull_requests: [26162]
---

`JsonSchemaBevyType::properties` is now a `BTreeMap<String, Value>` instead of a `HashMap`, so properties are sorted by name.
Each property in `registry.schema` output now has a `fieldIndex` key with the field's declaration order, and a `typePath` key with the field's type path.

The `typePath` of a property inside an enum struct variant used to hold the field name.
It now holds the field's type path, like `"f32"`. Use the property's key to get the field name.
