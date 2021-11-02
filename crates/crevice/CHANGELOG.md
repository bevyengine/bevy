# Crevice Changelog

## Unreleased Changes

## [0.8.0] - 2021-10-26
* Added support for many math libraries directly: ([#37])
	* cgmath 0.18, behind the `cgmath` feature
	* nalgebra 0.29, behind the `nalgebra` feature
	* glam 0.19, behind the `glam` feature
* Added support for generating GLSL source from structs. ([#33])
* Fixed many, many subtle alignment bugs. ([#28], [#35])
* Disabled bool-based types temporarily ([#36])
* Renamed `AsStdN::StdNType` to `Output`.
* Increased MSRV to 1.52.1.

[#28]: https://github.com/LPGhatguy/crevice/issues/28
[#33]: https://github.com/LPGhatguy/crevice/pull/33
[#35]: https://github.com/LPGhatguy/crevice/pull/35
[#36]: https://github.com/LPGhatguy/crevice/issues/36
[#37]: https://github.com/LPGhatguy/crevice/pull/37
[0.8.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.8.0

## [0.7.1] - 2021-07-24
* Fixed broken crates.io release by publishing crevice-derive too.

[0.7.1]: https://github.com/LPGhatguy/crevice/releases/tag/v0.7.1

## [0.7.0] - 2021-07-20 (Yanked)
**This release was yanked due to an issue reported in [#32].**

* Added `ivec`, `uvec`, and `bvec` structs ([#18])
* Improved padding behavior for structs and matrices ([#20])
* Implemented Crevice traits for more types ([#23])
* Added conversions from Std* types to AsStd* types ([#24])
* Added no_std support ([#25])

[#18]: https://github.com/LPGhatguy/crevice/pull/18
[#20]: https://github.com/LPGhatguy/crevice/pull/20
[#23]: https://github.com/LPGhatguy/crevice/pull/23
[#24]: https://github.com/LPGhatguy/crevice/pull/24
[#25]: https://github.com/LPGhatguy/crevice/pull/25
[#32]: https://github.com/LPGhatguy/crevice/issues/32
[0.7.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.7.0

## [0.6.0] - 2021-02-24
* Added `std430` support. Most APIs between `std140` and `std430` are the same!
* Added the `WriteStd140` trait. This trait is more general than `AsStd140` and is automatically implemented for all existing `AsStd140` implementers.
* Added `Writer::write_std140` to write a type that implements `Std140`.
* Added `AsStd140::std140_size_static`. This is similar to the old size method, `std140_size`, but no longer requires a value to be passed. For size measurements that depend on a value, use `WriteStd140::std140_size` instead.
* Deprecated `Writer::write_slice`, as `Writer::write` now accepts slices.
* Changed bounds of some functions, like `Writer::write` to use `WriteStd140` instead of `AsStd140`. This should affect no existing consumers.
* Moved `std140_size` from `AsStd140` to `WriteStd140`. Some existing consumers may need to import the other trait to access this m ethod.

[0.6.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.6.0

## [0.5.0] - 2020-10-18
* Added f64-based std140 types: `DVec2`, `DVec3`, `DVec4`, `DMat2`, `DMat3`, and `DMat4`.
* Added support for std140 structs with alignment greater than 16.
* Fixed padding for std140 matrices; they were previously missing trailing padding.

[0.5.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.5.0

## [0.4.0] - 2020-10-01
* Added `AsStd140::std140_size` for easily pre-sizing buffers.
* `Writer::write` and `Sizer::add` now return the offset the value is or would be written to.
* Added `std140::DynamicUniform` for aligning dynamic uniform members.
* Added `Writer::write_slice` for writing multiple values in a row.

[0.4.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.4.0

## [0.3.0] - 2020-09-22
* Added `Std140::as_bytes`, reducing the need to work with bytemuck directly.
* Removed public re-export of bytemuck.

[0.3.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.3.0

## [0.2.0] - 2020-09-22
* Added documentation for everything in the crate.
* Removed `type_layout` being exposed except for internal tests.
* Fixed alignment offset not taking into account previously added alignment.
* Added `std140::Writer`, for writing dynamically laid out types to buffers.
* Added `std140::Sizer`, for pre-calculating buffer sizes.

[0.2.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.2.0

## [0.1.0] - 2020-09-18
* Initial MVP release

[0.1.0]: https://github.com/LPGhatguy/crevice/releases/tag/v0.1.0
