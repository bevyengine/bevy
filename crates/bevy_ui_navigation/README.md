# Bevy cursor navigation

An ECS implementation of a UI navigation algorithm.
Generally useful for gamepad UI navigation.
It also provides a menu navigation system.

This crate is generic over the input and UI.
It makes no assumptions as to what it is used for,
beside the presence of bevy's ECS.

Bevy provides default inputs and UI in the [`bevy_ui`](../bevy_ui) crate.

This implementation follows the specification of [RFC 41][rfc41],
and is a fork of the [bevy-ui-navigation] crate.

## Examples

Check the [`ui_navigation`][examples] directory
in the `examples` directory at the root of the bevy crate.

[rfc41]: https://github.com/bevyengine/rfcs/pull/41
[examples]: https://github.com/bevyengine/bevy/tree/main/examples/ui_navigation
[bevy-ui-navigation]: https://lib.rs/crates/bevy-ui-navigation
