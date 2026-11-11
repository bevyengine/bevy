---
title: "`Val` arithmetic now panics on keyword variants"
pull_requests: [26487]
---

`Val`'s impls for `Add`, `Sub`, `Mul`, `MulAssign`, `Div`, `DivAssign` and `Neg` now panic on keyword arguments, instead of failing silently.

`TryStableInterpolate` returns a `ValArithmeticError` instead of importing and using `MismatchedUnitsError`. Attempting to interpolate between keywords, even the same keywords, now returns a `ValArithmeticError::NonEvaluable` error.

`try_add` and `try_sub` now return a `ValArithmeticError::NonEvaluable` error if given a keyword argument, instead of returning `ValArithmeticError::IncompatibleUnits`.
