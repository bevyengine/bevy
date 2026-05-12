---
title: "`custom_attributes` methods now return `Option`"
pull_requests: [24171]
---

Various `custom_attributes` methods now return `Option<&CustomAttributes>`.
Previously they returned `&CustomAttributes`.

This is a memory optimization for types that do not have have any custom
attributes - they will return `None` instead of a reference to an empty
`CustomAttributes`.

The affected methods are:

- `NamedField::custom_attributes`
- `UnnamedField::custom_attributes`
- `StructInfo::custom_attributes`
- `TupleStructInfo::custom_attributes`
- `EnumInfo::custom_attributes`
- `VariantInfo::custom_attributes`
- `StructVariantInfo::custom_attributes`
- `TupleVariantInfo::custom_attributes`
- `UnitVariantInfo::custom_attributes`
