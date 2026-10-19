---
title: Number input precision in display units, and percentages
authors: ["@viridia"]
pull_requests: [26158]
---

A `FeathersNumberInput` with `NumberInputUnits` stores its value in one unit and shows it in another: angles are stored in radians but shown in degrees.
`NumberInputPrecision` and `NumberInputStep` are now measured in the units the user sees.
Before, a precision of 2 on an angle rounded to hundredths of a radian, so dragging showed values like `17.761973°`.
Now it rounds to hundredths of a degree and shows `17.76°`.
Dragging speed is worked out in display units too, so an angle field moves by sensible amounts of a degree.

Values converted between units also display without floating point noise, so an `f32` angle of 0.01° no longer shows as `0.0099999998°`.

There is also a new built-in unit, `RatioPercent` (`ratio_percent`), for fractions shown as percentages: a value of 0.4 shows as `40%`, and typing `40` or `40%` stores 0.4.
Combined with display-unit precision, `NumberInputPrecision(1)` rounds to tenths of a percent:

```rust
bsn! {
    @FeathersNumberInput
    NumericValue::F32(0.5)
    NumberInputUnits::new(&RatioPercent)
    NumberInputPrecision(1)
}
```

Custom units can do the same.
`UnitsFormat` now has `canonical_to_display` and `display_to_canonical` methods for converting between stored and display units, and `format_display_value` for formatting a converted value without noise.
For units that are just a scale factor, implementing `StandardUnitKind` with a table of units is all you need.
