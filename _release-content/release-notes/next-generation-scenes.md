---
title: "Next Generation Scenes"
authors: ["@cart"]
pull_requests: [23413, 23880, 23808, 23905, 24008]
---

**Bevy 0.19** introduces our brand new, massively improved scene system for Bevy. We've been working on this for a _long time_ (years now!), and we are excited to finally get it in the hands of Bevy developers. It makes defining scenes in code (and ultimately in assets produced by the upcoming Bevy Editor) much nicer.

### BSN (Bevy Scene Notation)

BSN is an ergonomic Rust-like scene syntax which can be defined in Rust code via the `bsn!` macro _and_ in `.bsn` asset files. If you were ever bothered by the verbosity and complexity of spawning complex collections of entities in Bevy, you will probably enjoy what BSN has to offer. BSN can be used to spawn anything in the ECS. This benefits all scenarios, but it is worth calling out explicitly that this makes Bevy UI code significantly easier to read and write.

Note that while **Bevy 0.19** supports scene assets, we aren't yet shipping a first-party `.bsn` asset loader. **Bevy 0.19** focuses on the code-driven workflow, and we plan to roll out the asset driven workflow in the next release.

The new scene system is flexible and format-independent: BSN is our recommended default format, but third parties are free to build their own, and we plan to make formats like `glTF` directly compatible. 

A `bsn!` expression is essentially a list of components to add to an entity:

```rust
bsn! {
    Player {
        score: 0
    }
    Team::Blue
}
```

So far this looks and behaves much like Bevy's existing `Bundle` (which is _just_ a collection of components). But BSN has a ton of additional superpowers.

### Optional Fields

In BSN, you don't need to specify every field, or use `..Default::default()`. You only need to set the fields you care about, and the rest will have their default values:

```rust
#[derive(Component, Default, Clone)]
struct Player {
    score: usize,
    coins: usize,
}

bsn! {
    Player {
        score: 0
    }
}
```

You can also just specify the type name if you want all the fields to take on their default values:

```rust
bsn! {
    Player
}
```

Fields values can be arbitrary Rust expressions via `{}` syntax:

```rust
bsn! {
    Player { score: {current_points + 10} }
}
```

### BSN Relationships

BSN has first-class support for ECS Relationships. You can spawn related entities (such as children) inline:

```rust
bsn! {
    Player
    Children [
        Sword,
        Shield,
    ]
}
```

This also works for custom relationships:

```rust
bsn! {
    Player
    Inventory [
        Apple,
        Potion,
    ]
}
```

### Scene Functions

You can define reusable BSN functions like this:

```rust
fn player() -> impl Scene {
    bsn! {
        Player
        Children [ Sword, Shield ]
    }
}
```

These can accept and use parameters:

```rust
fn player(name: &str) -> impl Scene {
    bsn! {
        Name(name)
        Player
    }
}
```

### Scenes are Composable Patches

A BSN expression is a "patch", it does not write a "full" instance of every type it defines. This means you can layer scenes on top of each other:

```rust
fn button() -> impl Scene {
    bsn! {
        Button
        Node { width: px(100) }
    }
}

fn my_button() -> impl Scene {
    bsn! {
        button()
        Node { height: px(100) }
    }
}
```

`my_button` will spawn with a `Node { width: px(100), height: px(100) }` component. Components in scenes are initialized to their defaults, and each additional scene layer writes its fields on top of those defaults.

### Scene Assets and Caching

While **Bevy 0.19** doesn't ship with an official `.bsn` asset loader, it _does_ already functionally support scene asset dependencies. We just don't yet include any built-in loaders for them:

```rust
commands.queue_spawn_scene(bsn! {
    :"player.bsn"
    Transform {
        translation: Vec3 { x: 10. }
    }
})
```

This (if there was a `.bsn` asset loader) would spawn a scene that includes the `"player.bsn"` scene asset and patches the "x position" to be `10`. BSN is dependency-aware: if you use `queue_spawn_scene` instead of `spawn_scene`, it will wait to spawn the scene until all dependencies have loaded. `spawn_scene` will always try to spawn the scene immediately ... if it has scene dependencies that aren't loaded yet it will fail.

Also note the `:`, which is "caching" syntax. When first loaded, this will resolve the `"player.bsn"` scene and cache the results for reuse. This makes spawning multiple instances of the scene much cheaper, as it only needs to resolve whatever is layered "on top" of the cached scene.

We're [working](https://github.com/bevyengine/bevy/pull/23576) on an official `.bsn` asset loader, and we also plan on porting Bevy's glTF loader to the new scene system (so you can depend on `"my_scene.gltf"` just like you would a `my_scene.bsn` file). The `bsn!` macro and spawning system already supports scene assets, so if you're feeling adventurous you can try implementing your own Bevy scene format while you wait for ours!

### Scene Lists

`bsn!` / `Scene` corresponds to a single entity. `bsn_list!` / `SceneList` is the same idea, but applied to lists of entities:

```rust
fn players() -> impl SceneList {
    bsn_list! [
        (#Player1 Team::Blue),
        (#Player2 Team::Red),
    ]
}
```

Entities in a `bsn_list!` are comma separated, and the parentheses to visually indicate entity boundaries are optional:

```rust
fn players() -> impl SceneList {
    bsn_list! [
        #Player1 Team::Blue,
        #Player2 Team::Red,
    ]
}
```

The "BSN relationship syntax" seen above (ex: `Children []`) uses `SceneList`. This means you can pass scene lists as arguments to your scenes:

```rust
fn widget(children: impl SceneList) -> impl Scene {
    bsn! {
        Widget
        Children [ {children} ]
    }
}
```

### Observing Events

`bsn!` entities can easily observe events, making it easy to embed "callback-style" behaviors in your scenes:

```rust
fn button() -> impl Scene {
    bsn! {
        Node { width: px(100), height: px(50) }
        on(|press: On<Pointer<Press>>| {
            info!("button pressed!")
        })
    }
}
```

### Templates

A BSN expression actually defines "templates" for components rather than the actual components themselves. A `Template` is essentially a fancy constructor for a type, which produces an output type (such as a Component). Critically, `Template` has access to the `World`, the current entity, and the "scene spawn context". This enables powerful behaviors, such as loading assets from a given asset path and producing asset handles (ex: `Handle<Image>`).

The "old" approach to spawning via bundles required passing in every ECS dependency into a bundle function and manually using that dependency to produce the final value:

```rust
fn player(asset_server: &AssetServer) -> impl Bundle {
    (
        Player {
            score: 10,
            ..Default::default()
        },
        children! [
            Sprite {
                image: asset_server.load("player.png"),
                ..Default::default()
            }
        ]
    )
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(player(&asset_server))
}
```

This gets _quite_ nasty when spawning complex deeply nested scenes with many dependencies.

BSN makes this all much easier:

```rust
fn player() -> impl Scene {
    bsn! {
        Player { score: 10 }
        Children [
            Sprite { image: "player.png" }
        ]
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_scene(player());
}
```

Spawning a scene no longer requires knowing every little dependency it requires internally, and common actions like loading and assigning assets via their paths is simple!

This does mean that BSN requires types to have a `Template`. This is accomplished via the `FromTemplate` trait, which tells BSN what `Template` type it should use for a given `Component`. `FromTemplate` can be derived, which will also generate a `Template` type for your type. Fortunately, most types _do not_ need to derive or implement `FromTemplate` manually. This is because `FromTemplate` and `Template` is automatically implemented for every type that implements `Default` and `Clone`. These types are "templates of themselves" and are just "passed through". You only need to derive `FromTemplate` if you need template features (such as the `Sprite` use case above, which uses a `Handle<Image>` template to accept `"player.png"`).

### Inline Asset Templates

BSN ships with support for "inline assets" via the `asset_value` template:

```rust
fn cube() -> impl Scene {
    bsn! {
        Mesh3d(asset_value(Cuboid::new(1., 1., 1.)))
    }
}
```

Compare that to what was necessary before!

```rust
fn setup(meshes: Res<Assets<Meshes>>) -> impl Bundle {
    let handle = meshes.add(Cuboid::new(1., 1., 1.));
    Mesh3d(handle)
}
```

### Entity Reference Syntax

BSN has special "entity reference syntax" to define an Entity's `Name` component:

```rust
bsn! {
    #FirstPlayer
    Player
}
```

This is essentially the same as:

```rust
bsn! {
    Name("FirstPlayer")
    Player
}
```

However "entity reference syntax" also enables referencing that entity elsewhere in the scene:

```rust
#[derive(Component, FromTemplate)]
struct Reference(Entity);

bsn! {
    #Root
    Children [
        Reference(#Root)
    ]
}
```

You can access _any_ entity reference defined in a given `bsn! {}` scope anywhere else in that scope:

```rust
bsn! {
    References {
        child: #Child,
        grandchild: #Grandchild,
    }
    Children [
        #Child Children [
            #Grandchild
        ]
    ]
}
```

In the context of `bsn_list!`, this enables defining graph structures:

```rust
bsn_list! [
    (#A PointsTo(#B)),
    (#B PointsTo(#A)),
]
```

### Implicit Into

Most values in "field position" support "implicit `.into()`". This means types that can convert into other types can generally skip manual conversion:

```rust
#[derive(Component, Default, Clone)]
struct Foo(String);

bsn! {
    Foo("hello")
}
```

This works because `"hello"` is a `&str`, which has an `Into<String>` implementation. This is especially nice in the context of defining Bevy UI values:

```rust
// Raw Rust
Node {
    border: UiRect::all(Val::Px(2.0))
    ..Default::default()
}

// BSN
Node { border: px(2) }
```

`px(2)` is just a function that produces a `Val::Px(2.0)`, and `UiRect` has an `Into` impl for `Val`, which produces `UiRect::all` (writes the value to all four border "sides"). The ergonomics here are competitive with things like CSS, but it is fully statically typed and derived from normal Rust trait conversions (these aren't special cased / hard-coded). This means you can build your own!

### Scene Components

It has almost been a Bevy developer right of passage to define something like a `Player` component, which has complex behaviors that rely on some larger "scene", and then ask questions like "how to I spawn this all together?" and "how do I write code that can safely assume the whole scene is present?". Bevy developers have solved these problems in a variety of creative ways, but there has never been an easy recommended / idiomatic upstream solution.

BSN solves this problem by making it possible to associate a `Scene` with a `Component` via the `SceneComponent` derive:

```rust
#[derive(SceneComponent, Default, Clone)]
struct Player {
    score: usize
}

impl Player {
    fn scene() -> impl Scene {
        bsn! {
            Transform { translation: Vec3 { x: 10. } }
            Children [
                LeftHand,
                RightHand,
            ]
        }
    }
}
```

Scene components can then be spawned like this:

```rust
world.spawn_scene(bsn! {
    @Player { score: 10 }
})
```

Scene Components must be spawned this way (as a "scene component"), and will log errors if they are spawned directly (ex: via `world.spawn(Player::default())`). Critically, this provides the guarantee that if the `Player` component is present, the full scene will also be present at spawn time. As a developer this means you can write code that queries for `Player` and assume that it will have both a `LeftHand` and a `RightHand` child (provided they haven't been removed since being spawned). This was a major missing piece in the Bevy data model!

Scene Components can also define "props" which are passed into the scene function and can inform BSN outputs:

```rust
#[derive(SceneComponent, Default, Clone)]
#[scene(PlayerProps)]
struct Player {
    score: usize,
}

#[derive(Default)]
struct PlayerProps {
    alignment: Alignment
}

impl Player {
    fn scene(props: PlayerProps) -> impl Scene {
        let alignment: Box<dyn Scene> = match props.alignment {
            Alignment::Good => Box::new(bsn! { AngelWings }),
            Alignment::Evil => Box::new(bsn! { DevilHorns }),
        };
        bsn! {
            #Player
            alignment
            Items [ Sword, Shield ]
        }
    }
}

bsn! {
    @Player {
        // this is a "prop"
        @alignment: Alignment::Good,
        // this is a normal field
        score: 10,
    }
}
```

"Props" are evaluated first (before component field patches). Logically, they are evaluated immediately / in-place and the SceneComponent's scene is immediately applied to the current scene. This means the scene they produce can be patched. This _also_ means that you cannot patch "props", as they do not exist later in the scene.

The `SceneComponent` derive also supports shorthand for scene assets:

```rust
#[derive(SceneComponent, Default, Clone)]
#[scene("player.bsn")]
struct Player {
    score: usize
}
```

Again, note that **Bevy 0.19** does not ship with a `.bsn` asset loader. We're working on it!

The `SceneComponent` derive looks for the `Player::scene` function by default, but you can specify a custom function too:

```rust
#[derive(SceneComponent, Default, Clone)]
#[scene(player)]
struct Player {
    score: usize
}

fn player() -> impl Scene {
    bsn! { Player }
}
```

### Scene Spawning Systems

**Bevy 0.19** ships with a helper to easily spawn scene functions. This is a _fully self-contained_ Bevy app that spawns a 2D scene:

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, level.spawn())
        .run();
}

fn level() -> impl SceneList {
    bsn_list![
        Camera2d,
        Sprite { image: "player.png" }
    ]
}
```

`.spawn()` will turn any function that returns a `Scene` or a `SceneList` into a system that spawns that scene.
