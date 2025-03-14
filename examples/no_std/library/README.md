# Bevy `no_std` Compatible Library

This example demonstrates how to create a `no_std`-compatible library crate for use with Bevy.
For the sake of demonstration, this library adds a way for a component to be added to an entity after a certain delay has elapsed.
Check the [Cargo.toml](Cargo.toml) and [lib.rs](src/lib.rs) for details around how this is implemented, and how we're able to make a library compatible for all users in the Bevy community.
