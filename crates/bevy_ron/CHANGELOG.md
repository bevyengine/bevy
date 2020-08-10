# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Next
- Added `decimal_floats` PrettyConfig option, which always includes decimals in floats (`1.0` vs `1`) ([#237](https://github.com/ron-rs/ron/pull/237))

## [0.6.0] - 2020-05-21
### Additions
- Implement integer support in Numbers ([#210](https://github.com/ron-rs/ron/pull/210))
- Port `ser::Serializer` to `io::Write` ([#206](https://github.com/ron-rs/ron/pull/206))
- Support i128 and u128 ([#219](https://github.com/ron-rs/ron/pull/219))
- Allow pretty ser to work with implicit-some extension ([#182](https://github.com/ron-rs/ron/pull/182))
- Make PrettyConfig future-proof ([#173](https://github.com/ron-rs/ron/pull/173))
- Use indexmap to preserve order (optional) ([#172](https://github.com/ron-rs/ron/pull/172))
- Add tests for different enum representations ([#166](https://github.com/ron-rs/ron/pull/166))
- Implement inf, -inf and NaN handling ([#163](https://github.com/ron-rs/ron/pull/163))
- Add VS code language tooling ([#160](https://github.com/ron-rs/ron/pull/160))
- Be smarter about integer deserialization ([#157](https://github.com/ron-rs/ron/pull/157))

### Fixes
- Fix parsing of borrowed strings ([#228](https://github.com/ron-rs/ron/pull/228))
- Fix depth limit test for off-by-one fix ([#225](https://github.com/ron-rs/ron/pull/225))
- Remove deprecated uses of `Error::description` ([#208](https://github.com/ron-rs/ron/pull/208))
- Preserve ordering of map sequences ([#197](https://github.com/ron-rs/ron/pull/197))
- Remove unneeded Neg requirement for signed_integer ([#193](https://github.com/ron-rs/ron/pull/193))
- Ensure "Untagged tuple-like enum variants not deserializing correctly……" is fixed ([#170](https://github.com/ron-rs/ron/pull/170))

### Changes
- Update `serde` requirement to 1.0.60 ([#226](https://github.com/ron-rs/ron/pull/226))
- Replace Travis with GitHub actions ([#223](https://github.com/ron-rs/ron/pull/223))
- Rename `format_doc_comments` to `format_code_in_doc_comment`
- Update base64 requirement from 0.11 to 0.12 ([#204](https://github.com/ron-rs/ron/pull/204))
- Update base64 requirement from 0.10 to 0.11 ([#195](https://github.com/ron-rs/ron/pull/195))
- Update `serde_bytes` to 0.11 ([#164](https://github.com/ron-rs/ron/pull/164))

## [0.5.1] - 2019-04-05
### Fixes
- Increase source compability from Rust `1.34.0` to `1.31.0` by not relying on `as _` imports ([#156](https://github.com/ron-rs/ron/pull/156))

## [0.5.0] - 2019-03-31
### Additions
- Don't insert new lines in empty arrays or maps ([#150](https://github.com/ron-rs/ron/pull/150))
### Changes
- Transition to Rust 2018 ([#149](https://github.com/ron-rs/ron/pull/149))

## [0.4.2] - 2019-03-01
### Additions
- Add integer check for deserializer ([#148](https://github.com/ron-rs/ron/pull/148))
- Implement `Value::into_rust` ([#146](https://github.com/ron-rs/ron/pull/146))

## [0.4.1] - 2019-01-09
### Additions
- Allow underscores in integers ([#135](https://github.com/ron-rs/ron/pull/135))
- Added extension documentation ([#130](https://github.com/ron-rs/ron/pull/130))
### Changes
- Move sublime text syntax to separate repo ([#138](https://github.com/ron-rs/ron/pull/138))
- Update `base64` crate dependency to 0.10 ([#137](https://github.com/ron-rs/ron/pull/137))

## [0.4.0] - 2018-08-11
### Fixes
- Handle tuple deserialization in deserialize_any properly ([#124](https://github.com/ron-rs/ron/pull/124))
### Changes
- Add raw string syntax to grammar ([#125](https://github.com/ron-rs/ron/pull/125))
- Reexport `Value` at root ([#120](https://github.com/ron-rs/ron/pull/120))

## [0.3.0] - 2018-06-15
### Additions
- `serde_bytes` fields to be encoded using base64. ([#109](https://github.com/ron-rs/ron/pull/109))
### Fixes
- Allow raw string literals ([#114](https://github.com/ron-rs/ron/pull/114))
### Changes
- Now depends on `base64` 0.9.2.

## [0.2.2] - 2018-05-19
### Fixes
- Allow whitespace in newtype variants ([#104](https://github.com/ron-rs/ron/pull/104))

## [0.2.1] - 2018-05-04
### Additions
- Add multi-line comments ([#98](https://github.com/ron-rs/ron/pull/98))
### Fixes
- Allow more whitespace inside newtypes ([#103](https://github.com/ron-rs/ron/pull/103))

## [0.2.0] - 2018-02-14
### Additions
- Limit the pretty depth ([#93](https://github.com/ron-rs/ron/pull/93))
- Add support for `\x??` and improve unicode escapes ([#84](https://github.com/ron-rs/ron/pull/84))

## [0.1.7] - 2018-01-24
### Additions
- Deep array indexing ([#88](https://github.com/ron-rs/ron/pull/88))
- Pretty sequence indexing ([#86](https://github.com/ron-rs/ron/pull/86))
- Add unicode support for chars ([#80](https://github.com/ron-rs/ron/pull/80))
- Add support for hex, oct and bin numbers ([#78](https://github.com/ron-rs/ron/pull/78))
- Allow implicit Some ([#75](https://github.com/ron-rs/ron/pull/75))
- Add grammar specification ([#73](https://github.com/ron-rs/ron/pull/73))
- Add extension support and first extension, unwrap_newtypes ([#72](https://github.com/ron-rs/ron/pull/72))
### Fixes
- Directly serialize `f32` ([#81](https://github.com/ron-rs/ron/pull/81))

## [0.1.6] - 2018-01-24
### Additions
- Implement sequence indexing ([#87](https://github.com/ron-rs/ron/pull/87))
### Fixes
- Remove ident variable from Sublime syntax ([#71](https://github.com/ron-rs/ron/pull/71))

## [0.1.5] - 2017-12-27
### Additions
- Allow creating a new serializer ([#70](https://github.com/ron-rs/ron/pull/70))
- Sublime syntax highlighter ([#67](https://github.com/ron-rs/ron/pull/67))
- Add support for integers ([#65](https://github.com/ron-rs/ron/pull/65))
- Implement `Deserializer` for `Value` ([#64](https://github.com/ron-rs/ron/pull/64))

## [0.1.4] - 2017-10-12
### Additions
- Add `PrettyConfig` ([#61](https://github.com/ron-rs/ron/pull/61))
- impl `deserialize_ignored_any` for `id` ([#60](https://github.com/ron-rs/ron/pull/60))
### Fixes
- Fix  deserializing of ignored fields ([#62](https://github.com/ron-rs/ron/pull/62))

## [0.1.3] - 2017-10-06
### Fixes
- Removed indentation from tuple variant pretty encoder ([#57](https://github.com/ron-rs/ron/pull/57))

## [0.1.2] - 2017-10-06
### Fixes
- Fix decoding of string literals ([#56](https://github.com/ron-rs/ron/pull/56))
- Add `Value` and implement `deserialize_any` ([#53](https://github.com/ron-rs/ron/pull/53))

## [0.1.1] - 2017-08-07
### Fixes
- Be more permissive wrt whitespace decoding ([#41](https://github.com/ron-rs/ron/pull/41))
### Additions
- Add utility function to deserialize from `std::io::Read` ([#42](https://github.com/ron-rs/ron/pull/42))

## [0.1.0] - 2015-08-04
### Changes
- Reorganize deserialization modules ([#30](https://github.com/ron-rs/ron/pull/30))
- Rework deserializer not to require `pom` crate [#27](https://github.com/ron-rs/ron/pull/27), ([#38](https://github.com/ron-rs/ron/pull/38))
- Dual license under Apache 2.0 and MIT ([#26](https://github.com/ron-rs/ron/pull/26))
### Fixes
- Use CRLF for serializatio on Windows ([#32](https://github.com/ron-rs/ron/pull/32))
- Fix bors-ng to work with travis ([#31](https://github.com/ron-rs/ron/pull/31))
- Handle escapes ([#23](https://github.com/ron-rs/ron/pull/23))
### Additions
- Improve error reporting ([#29](https://github.com/ron-rs/ron/pull/29))
- Allow decoding of comments ([#28](https://github.com/ron-rs/ron/pull/28))
- Add `pretty` option to serializer ([#25](https://github.com/ron-rs/ron/pull/25))
- Add roundtrip tests ([#24](https://github.com/ron-rs/ron/pull/24))

## [0.0.1] - 2015-07-30
Initial release