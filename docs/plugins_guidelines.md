# Third Party Plugin Guidelines

Bevy has a plug and play architecture, where you can easily add plugins for new features, or replace built-in plugins with your own.

This document targets plugin authors.

## Naming

You are free to use a `bevy_xxx` name for your plugin, with the caveat "please be reasonable". If you are about to claim a generic name like `bevy_animation`, `bevy_color`, or `bevy_editor`... please ask first. The rational is explained [here](https://github.com/bevyengine/bevy/discussions/1202#discussioncomment-258907).

## Bevy version
## Promotion

You can promote your plugin to Bevy's [communities](https://github.com/bevyengine/bevy#community):

* Add it to [Awesome Bevy](https://github.com/bevyengine/awesome-bevy)
* Announce it on [Discord](https://discord.gg/gMUk5Ph), in channel `#showcase`
* Announce it on [Reddit](https://reddit.com/r/bevy)

Indicating which version of your plugin works with which version of bevy can be a great help for your users. Some of your user may be using an older version of bevy for any number of reason, you can help them finding which version of your plugin they should use. This can be shown as a simple table in your readme with each version of bevy with a working version of your plugin.

|bevy|bevy_awesome_plugin|
|---|---|
|0.4|0.3|
|0.3|0.1|

## Bevy features

You should disable Bevy features that you don't use. This is because with Cargo, features are additives, meaning that features that are enabled in bevy in your plugin can't be disabled by someone using your plugin. You can find the list of features [here](cargo_features.md).
```
bevy = { version = "0.4", default-features = false, features = ["..."] }
```

## Master tracking

If you intend to track Bevy's master, you can specify the latest commit you support in your cargo.toml file:
```
bevy = { version = "0.4", git = "https://github.com/bevyengine/bevy", rev="509b138e8fa3ea250393de40c33cc857c72134d3", default-features = false }
```
You can specify the dependency [both as a version and with git](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations), the version will be used if using the dependency from crates.io, the git dependency will be used otherwise.

Bevy is evolving very fast, and stating with a badge how you intend to track Bevy's master can be useful for your users.

|badge|description|image URL|
|-|-|-|
|![](https://img.shields.io/badge/Bevy%20tracking-master-green)|I intend to track master as much as I can|`https://img.shields.io/badge/Bevy%20tracking-master-green`|
|![](https://img.shields.io/badge/Bevy%20tracking-PR%20welcome-yellow)|I welcome PR that will update my plugin to current Bevy master|`https://img.shields.io/badge/Bevy%20tracking-PR%20welcome-yellow`|
|![](https://img.shields.io/badge/Bevy%20tracking-released%20version-blue)|I will only follow released Bevy's versions|`https://img.shields.io/badge/Bevy%20tracking-released%20version-blue`|


