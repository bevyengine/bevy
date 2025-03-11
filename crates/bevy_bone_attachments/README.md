# Bevy Bone Attachments

Bone attachments are used to accessorize a model with others. A common use is for example
to attach a weapon to a characters hands.

This relies on the parent model having `AnimationTarget` components and the attachment having
`AnimationTargetId` component.  

Currently this only works by attaching a `Scene` to another entity. If the `Scene` is loaded
from a `glTf`, use the `GltfLoaderSetting::include_animation_target_ids` setting to load the `AnimationTargetId`
of the attachment.

## Links

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_color.svg)](https://crates.io/crates/bevy_color)
[![Downloads](https://img.shields.io/crates/d/bevy_color.svg)](https://crates.io/crates/bevy_color)
[![Docs](https://docs.rs/bevy_color/badge.svg)](https://docs.rs/bevy_color/latest/bevy_color/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)
