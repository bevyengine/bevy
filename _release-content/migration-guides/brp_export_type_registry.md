---
title: "`export_type` takes a `TypeRegistry`"
pull_requests: [26171]
---

`bevy_remote::schemas::json_schema::export_type` now takes a `&TypeRegistry` as its last argument, used to serialize each type's default value into the new `JsonSchemaBevyType::default` field.

The `From<(&TypeRegistration, &SchemaTypesMetadata, &Components)>` implementation for `JsonSchemaBevyType` now takes the `&TypeRegistry` as a fourth tuple element.
