---
title: "Number input precision and step are measured in display units"
pull_requests: [26158]
---

`NumberInputPrecision` and `NumberInputStep` on a `FeathersNumberInput` are now measured in the units the user sees, not the units the value is stored in.
Previously, an angle field (stored in radians, shown in degrees) with `NumberInputPrecision(2)` snapped to 0.01 radians, so dragging showed values like `17.761973°`.
It now snaps to 0.01°.

If you set `NumberInputPrecision` or `NumberInputStep` on a field that has `NumberInputUnits` with a display unit different from the stored unit, convert the setting to display units.
`SoftLimit` and `HardLimit` are still in stored units.

To support this, the `UnitsFormat` trait now separates converting between units from formatting:

- New methods `canonical_to_display` and `display_to_canonical` convert between stored and display units. Both default to the identity.
- `format` now receives the value already converted to display units, as an `f64`, along with the stored `NumericFormat`: `fn format(&self, display: f64, fmt: NumericFormat, editing: bool) -> String`.
- `parse` now returns a value in display units: `fn parse(&self, text: &str) -> Result<f64, String>`.

Types that implement `StandardUnitKind` need no changes.
