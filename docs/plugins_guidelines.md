# Third Party Plugin Guidelines

Bevy has a plug and play architecture, where you can easily add plugins for new features, or replace built-in plugins with your own.

This document targets plugin authors.

## Checklist

* [ ] [Pick a reasonable, descriptive name](#naming)
* [ ] [Depend on Bevy git/main or latest release](#main-branch-tracking)
* [ ] [Pick a license](#licensing)
* [ ] [Keep your crate as small as possible](#small-crate-size)
* [ ] [Documentation and examples](#documentation-and-examples)
* [ ] [Indicate compatible plugin/Bevy versions](#indicate-compatible-versions)
* [ ] [Add cargo tests and CI](#tests-and-ci)
* [ ] [Publish your plugin](#publishing-your-plugin)
* [ ] [Promote your plugin](#promotion)

## Naming

You are free to use a `bevy_xxx` name for your plugin, but please be reasonable. If you are about to claim a generic name like `bevy_animation`, `bevy_color`, or `bevy_editor`, please ask first. The rationale is explained [here](https://github.com/bevyengine/bevy/discussions/1202#discussioncomment-258907).

## Main Branch Tracking

Bevy is evolving very fast. Regularly new features are working on the main branch, but are not yet released. Your plugin might depend on Bevy main or the latest release. You can also do both on different branches (e.g. have a `bevy_main` branch).

If you intend to track Bevy's main branch, you can specify the latest commit you support in your `Cargo.toml` file:

```toml
bevy = { version = "0.5", git = "https://github.com/bevyengine/bevy", rev="9788b386c7846c99978ab5c1a33698ec5a471d84", default-features = false }
```

You can specify the dependency [both as a version and with git](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations). The version will be used if the dependency is pulled from [crates.io](https://crates.io). Otherwise, the git dependency will be used.

You can use one of these badges to communicate to your users how closely you intend to track Bevy's main branch.

<!-- MD033 - The Badges could be downsized, without the inline HTML due to the large code column -->
<!-- markdownlint-disable-next-line MD033 -->
|<div style="width:100px">badge</div>|<div style="width:200px">description</div>|code|
|-|-|-|
|[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)|I intend to track main as much as I can|`[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-main-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)`|
|[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)|I will only follow released Bevy versions|`[![Bevy tracking](https://img.shields.io/badge/Bevy%20tracking-released%20version-lightblue)](https://github.com/bevyengine/bevy/blob/main/docs/plugins_guidelines.md#main-branch-tracking)`|

## Licensing

Bevy is dual licensed under [MIT or Apache 2.0](https://www.rust-lang.org/policies/licenses), at your option. Most other Rust projects (including Rust itself) also use this dual-license approach. MIT-only is very popular and you might be tempted to just use that (Bevy also used to be MIT-only), but there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both licenses. We highly recommend using the dual MIT / Apache-2.0 license for your Bevy Plugins and crates:

* Including the Apache 2.0 license option significantly reduces the difficulty and boilerplate of proper license compliance in published games because you only need to include one copy of the Apache 2.0 license.
* Provides maximum compatibility with Bevy and Rust, making it easier to upstream your changes.

## Small Crate Size

To avoid long build times in your plugin and in projects using it, you should aim for a small crate size:

* Disable Bevy features that you don't use
  * Features are additive => Bevy features enabled in your plugin cannot be disabled by someone using your plugin
  * You can find Bevy's features [here](cargo_features.md).

    ```toml
    bevy = { version = "0.5", default-features = false, features = ["..."] }
    ```

* Avoid large dependencies
* Put optional functionality and dependencies behind a feature

## Documentation and Examples

Documentation and examples are very useful for a crate.

In the case of a Bevy plugin, a few screenshots or movies/animated GIFs from your examples can really help to understand what your plugin is capable of.

Additionally, it can be helpful to list:

* SystemSets available from your plugin
  * if important, also mention their execution order
* Components available from your plugin

### Indicate Compatible Versions

Indicating which version of your plugin works with which version of Bevy can be helpful for your users. Some of your users may be using an older version of Bevy for any number of reasons. You can help them find which version of your plugin they should use. This can be shown as a simple table in your readme with each version of Bevy and the corresponding compatible version of your plugin.

| bevy | bevy_awesome_plugin |
|------|---------------------|
| 0.5  | 0.3                 |
| 0.4  | 0.1                 |

## Tests and CI

Tests are always good! For CI, you can check [this example](https://github.com/actions-rs/meta/blob/master/recipes/quickstart.md) for a quickstart using GitHub Actions. As Bevy has additional Linux dependencies, you should install them before building your project ([here is how Bevy is doing it](https://github.com/bevyengine/bevy/blob/9788b386c7846c99978ab5c1a33698ec5a471d84/.github/workflows/ci.yml#L40)). Even if you don't have many (or any) tests, setting up CI will compile check your plugin and ensure a basic level of quality.

## Publishing your Plugin

There are some [extra fields](https://doc.rust-lang.org/cargo/reference/manifest.html) that you can add to your `Cargo.toml` manifest, in the `[package]` section:

|field|description|
|-|-|
|[`description`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-description-field)|a description of the plugin|
|[`repository`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-repository-field)|URL of the plugin source repository|
|[`license`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields)|the plugin license|
|[`keywords`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-keywords-field)|keywords for the plugin - `"bevy"` at least is a good idea here|
|[`categories`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-categories-field)|categories of the plugin - see [the full list on crates.io](https://crates.io/categories)|
|[`exclude`](https://doc.rust-lang.org/cargo/reference/manifest.html#the-exclude-and-include-fields)|files to exclude from the released package - excluding the `assets` folder that you may have is a good idea, as well as any large file that are not needed by the plugin|

Once a crate is published to [crates.io](https://crates.io), there are two badges that you can add to your `README.md` for easy links:

|badge|code|
|-|-|
|[![crates.io](https://img.shields.io/crates/v/bevy)](https://crates.io/crates/bevy)|`[![crates.io](https://img.shields.io/crates/v/bevy_awesome_plugin)](https://crates.io/crates/bevy_awesome_plugin)`|
|[![docs.rs](https://docs.rs/bevy/badge.svg)](https://docs.rs/bevy)|`[![docs.rs](https://docs.rs/bevy_awesome_plugin/badge.svg)](https://docs.rs/bevy_awesome_plugin)`|

## Promotion

You can promote your plugin in Bevy's [communities](https://github.com/bevyengine/bevy#community):

* Add it as an [Asset on the official website](https://github.com/bevyengine/bevy-assets)
* Announce it on [Discord](https://discord.gg/bevy), in the `#crates` channel
* Announce it on [Reddit](https://reddit.com/r/bevy)
