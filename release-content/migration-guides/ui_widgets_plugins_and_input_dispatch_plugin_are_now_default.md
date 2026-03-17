---
title: "`UiWidgetsPlugins` and `InputDispatchPlugin` are now in `DefaultPlugins`"
pull_requests: [23346]
---

`UiWidgetsPlugins` and `InputDispatchPlugin` are now part of `DefaultPlugins`.

TODO: Why did we make this breaking change?

Remove `UiWidgetsPlugins` if you have `DefaultPlugins`

```rs
// 0.18
fn main() {
    App::new()
        .add_plugins(DefaultPlugins, UiWidgetsPlugins)
        .add_plugins((my_ambitious_game::game_plugin))
        .run();
}

// 0.19
fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Puff!
        .add_plugins((my_ambitious_game::game_plugin))
        .run();
}
```

Remove `InputDispatchPlugin` if you have `DefaultPlugins`

```rs
// 0.18
fn main() {
    App::new()
        .add_plugins(DefaultPlugins, UiWidgetsPlugins, InputDispatchPlugin)
        .add_plugins((my_sequel_game::game_plugin))
        .run();
}

// 0.19
fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Puff!
        .add_plugins((my_sequel_game::game_plugin))
        .run();
}
```

Remove `PopoverPlugin` if you have `DefaultPlugins`

```rs
// 0.18
fn main() {
    App::new()
        .add_plugins(DefaultPlugins, PopoverPlugin)
        .add_plugins((my_threequel_game::game_plugin))
        .run();
}

// 0.19
fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Puff!
        .add_plugins((my_threequel_game::game_plugin))
        .run();
}
```

Remove `ScrollbarPlugin` if you have `DefaultPlugins`

```rs
// 0.18
fn main() {
    App::new()
        .add_plugins(DefaultPlugins, ScrollbarPlugin)
        .add_plugins((my_fourquel_game::game_plugin))
        .run();
}

// 0.19
fn main() {
    App::new()
        .add_plugins(DefaultPlugins) // Puff!
        .add_plugins((my_fourquel_game::game_plugin))
        .run();
}
```
