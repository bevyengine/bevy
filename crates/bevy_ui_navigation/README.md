# Bevy UI navigation

The in-depth design specification is [available here][rfc41].

### Examples

Check out the [`examples`][examples] directory for bevy examples.

![Demonstration of "Ultimate navigation" example](https://user-images.githubusercontent.com/26321040/141612751-ba0e62b2-23d6-429a-b5d1-48b09c10d526.gif)


## Usage

See [this example][example-simple] for a quick start guide.

[The crate documentation is extensive][doc-root], but for practical reason
doesn't include many examples. This page contains most of the doc examples,
you should check the [examples directory][examples] for examples showcasing
all features of this crate.


### Simple case

To create a simple menu with navigation between buttons, simply replace usages
of [`ButtonBundle`] with [`FocusableButtonBundle`].

You will need to create your own system to change the color of focused elements, and add
manually the input systems, but with that setup you get: **Complete physical
position based navigation with controller, mouse and keyboard. Including rebindable
mapping**.

```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::DefaultNavigationPlugins;
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DefaultNavigationPlugins)
        .run();
}
```

Use the [`InputMapping`] resource to change keyboard and gamepad button mapping.

If you want to change entirely how input is handled, you should do as follow. All
interaction with the navigation engine is done through
[`EventWriter<NavRequest>`][`NavRequest`]:

```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::{NavigationPlugin, NavRequest};

fn custom_input_system_emitting_nav_requests(mut events: EventWriter<NavRequest>) {
    // handle input and events.send(NavRequest::FooBar)
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin::new())
        .add_system(custom_input_system_emitting_nav_requests)
        .run();
}
```

Check the [`examples directory`][examples] for more example code.

`bevy-ui-navigation` provides a variety of ways to handle navigation actions.
Check out the [`event_helpers`][module-event_helpers] module for more examples.

```rust
use bevy::{app::AppExit, prelude::*};
use bevy_ui_navigation::event_helpers::NavEventQuery;

#[derive(Component)]
enum MenuButton {
    StartGame,
    ToggleFullscreen,
    ExitGame,
    Counter(i32),
    //.. etc.
}

fn handle_nav_events(mut buttons: NavEventQuery<&mut MenuButton>, mut exit: EventWriter<AppExit>) {
    // Do something when player activates (click, press "A" etc.) a `Focusable` button.
    match buttons.single_activated_mut().deref_mut().ignore_remaining() {
        Some(MenuButton::StartGame) => {
            // start the game
        }
        Some(MenuButton::ToggleFullscreen) => {
            // toggle fullscreen here
        }
        Some(MenuButton::ExitGame) => {
            exit.send(AppExit);
        }
        Some(MenuButton::Counter(count)) => {
            *count += 1;
        }
        //.. etc.
        None => {}
    }
}
```

The focus navigation works across the whole UI tree, regardless of how or where
you've put your focusable entities. You just move in the direction you want to
go, and you get there.

Any [`Entity`] can be converted into a focusable entity by adding the [`Focusable`]
component to it. To do so, just:
```rust
# use bevy::prelude::*;
# use bevy_ui_navigation::Focusable;
fn system(mut cmds: Commands, my_entity: Entity) {
    cmds.entity(my_entity).insert(Focusable::default());
}
```
That's it! Now `my_entity` is part of the navigation tree. The player can select
it with their controller the same way as any other [`Focusable`] element.

You probably want to render the focused button differently than other buttons,
this can be done with the [`Changed<Focusable>`][Changed] query parameter as follow:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::{FocusState, Focusable};

fn button_system(
    mut focusables: Query<(&Focusable, &mut UiColor), Changed<Focusable>>,
) {
    for (focus, mut color) in focusables.iter_mut() {
        let new_color = if matches!(focus.state(), FocusState::Focused) {
            Color::RED
        } else {
            Color::BLACK
        };
        *color = new_color.into();
    }
}
```

### Snappy feedback

You will want the interaction feedback to be snappy. This means the
interaction feedback should run the same frame as the focus change. For this to
happen every frame, you should add `button_system` to your app using the
[`NavRequestSystem`] label like so:
```rust, no_run
use bevy::prelude::*;
use bevy_ui_navigation::{NavRequestSystem, NavRequest, NavigationPlugin};

fn custom_mouse_input(mut events: EventWriter<NavRequest>) {
    // handle input and events.send(NavRequest::FooBar)
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(NavigationPlugin::new())
        // ...
        // Add the button color update system after the focus update system
        .add_system(button_system.after(NavRequestSystem))
        // Add input systems before the focus update system
        .add_system(custom_mouse_input.before(NavRequestSystem))
        // ...
        .run();
}
// Implementation from earlier
fn button_system() {}
```


## More complex use cases

### Locking

If you need to supress the navigation algorithm temporarily, you can declare a
[`Focusable`] as [`Focusable::lock`].

This is useful for example if you want to implement custom widget with their
own controls, or if you want to disable menu navigation while in game. To
resume the navigation system, you'll need to send a [`NavRequest::Free`].


### `NavRequest::FocusOn`

You can't directly manipulate which entity is focused, because we need to keep
track of a lot of thing on the backend to make the navigation work as expected.
But you can set the focused element to any arbitrary `Focusable` entity with
[`NavRequest::FocusOn`].

```rust
use bevy::prelude::*;
use bevy_ui_navigation::NavRequest;

fn set_focus_to_arbitrary_focusable(
    entity: Entity,
    mut requests: EventWriter<NavRequest>,
) {
    requests.send(NavRequest::FocusOn(entity));
}
```

### Set the first focused element

You probably want to be able to chose which element is the first one to gain
focus. By default, the system picks the first [`Focusable`] it finds. To change
this behavior, spawn a dormant [`Focusable`] with [`Focusable::dormant`].

### `NavMenu`s

Suppose you have a more complex game with menus sub-menus and sub-sub-menus etc.
For example, in your everyday 2021 AAA game, to change the antialiasing you
would go through a few menus:
```text
game menu → options menu → graphics menu → custom graphics menu → AA
```
In this case, you need to be capable of specifying which button in the previous
menu leads to the next menu (for example, you would press the "Options" button
in the game menu to access the options menu).

For that, you need to use [`NavMenu`].

The high level usage of [`NavMenu`] is as follow:
1. First you need a "root" [`NavMenu`].
2. You need to spawn into the ECS your "options" button with a [`Focusable`]
   component. To link the button to your options menu, you need to do one of
   the following:
   * Add a [`Name("opt_btn_name")`][Name] component in addition to the
     [`Focusable`] component to your options button.
   * Pre-spawn the options button and store somewhere it's [`Entity` id][entity-id]
     (`let opt_btn = commands.spawn_bundle(FocusableButtonBundle).id();`)
3. to the `NodeBundle` containing all the options menu [`Focusable`] entities,
   you add the following bundle:
   * [`NavMenu::Bound2d.reachable_from_named("opt_btn_name")`][NavMenu::reachable_from_named]
     if you opted for adding the `Name` component.
   * [`NavMenu::Bound2d.reachable_from(opt_btn)`][NavMenu::reachable_from]
     if you have the [`Entity`] id.

In code, This will look like this:
```rust
use bevy::prelude::*;
use bevy_ui_navigation::{Focusable, NavMenu};
use bevy_ui_navigation::components::FocusableButtonBundle;

struct SaveFile;
impl SaveFile {
    fn bundle(&self) -> impl Bundle {
        // UI bundle to show this in game
        NodeBundle::default()
    }
}
fn spawn_menu(mut cmds: Commands, save_files: Vec<SaveFile>) {
    let menu_node = NodeBundle {
        style: Style { flex_direction: FlexDirection::Column, ..Default::default()},
        ..Default::default()
    };
    let button = FocusableButtonBundle::from(ButtonBundle {
        color: Color::rgb(1.0, 0.3, 1.0).into(),
        ..Default::default()
    });
    let mut spawn = |bundle: &FocusableButtonBundle, name: &'static str| {
          cmds.spawn_bundle(bundle.clone()).insert(Name::new(name)).id()
    };
    let options = spawn(&button, "options");
    let graphics_option = spawn(&button, "graphics");
    let audio_options = spawn(&button, "audio");
    let input_options = spawn(&button, "input");
    let game = spawn(&button, "game");
    let quit = spawn(&button, "quit");
    let load = spawn(&button, "load");

    // Spawn the game menu
    cmds.spawn_bundle(menu_node.clone())
        // Root NavMenu         vvvvvvvvvvvvvv
        .insert_bundle(NavMenu::Bound2d.root())
        .push_children(&[options, game, quit, load]);

    // Spawn the load menu
    cmds.spawn_bundle(menu_node.clone())
        // Sub menu accessible through the load button
        //                              vvvvvvvvvvvvvvvvvvvv
        .insert_bundle(NavMenu::Bound2d.reachable_from(load))
        .with_children(|cmds| {
            // can only access the save file UI nodes from the load menu
            for file in save_files.iter() {
                cmds.spawn_bundle(file.bundle()).insert(Focusable::default());
            }
        });

    // Spawn the options menu
    cmds.spawn_bundle(menu_node)
        // Sub menu accessible through the "options" button
        //                              vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
        .insert_bundle(NavMenu::Bound2d.reachable_from_named("options"))
        .push_children(&[graphics_option, audio_options, input_options]);
}
```

With this, your game menu will be isolated from your options menu, you can only
access it by sending [`NavRequest::Action`] when `options_button` is focused, or
by sending a [`NavRequest::FocusOn(entity)`][`NavRequest::FocusOn`] where `entity`
is any of `graphics_option`, `audio_options` or `input_options`.

Note that you won't need to manually send the [`NavRequest`] if you are using one
of the default input systems provided in the [`systems` module][module-systems].

Specifically, navigation between [`Focusable`] entities will be constrained to other
[`Focusable`] that are children of the same [`NavMenu`]. It creates a self-contained
menu.

### Types of `NavMenu`s

A [`NavMenu`] doesn't only define menu-to-menu navigation, but it also gives you
finner-grained control on how navigation is handled within a menu:
* `NavMenu::Wrapping*` (as opposed to `NavMenu::Bound*`) enables looping
  navigation, where going offscreen in one direction "wraps" to the opposite
  screen edge.
* `NavMenu::*Scope` creates a "scope" menu that catches [`NavRequest::ScopeMove`]
  requests even when the focused entity is in another sub-menu reachable from this
  menu. This behaves like you would expect a tabbed menu to behave.

See the [`NavMenu`] documentation or the ["ultimate" menu navigation
example][example-ultimate] for details.


#### Marking

If you need to know from which menu a [`NavEvent::FocusChanged`] originated, you
can use one of the [`marking`][module-marking] methods on the `NavMenu` seeds.

A usage demo is available in [the `marking.rs` example][example-marking].


## Changelog

* `0.8.2`: Fix offsetting of mouse focus with `UiCamera`s with a transform set
  to anything else than zero.
* `0.9.0`: Add [`Focusable::cancel`] (see documentation for details); Add warning
  message rather than do dumb things when there is more than a single [`NavRequest`]
  per frame
* `0.9.1`: Fix #8, Panic on diagonal gamepad input
* `0.10.0`: Add the `bevy-ui` feature, technically this includes breaking
  changes, but it is very unlikely you need to change your code to get it
  working 
  * **Breaking**: if you were manually calling `default_mouse_input`, it now has 
    additional parameters
  * **Breaking**: `ui_focusable_at` and `NodePosQuery` now have type parameters
* `0.11.0`: Add the `Focusable::lock` feature. A focusable now can be declared
  as "lock" and block the ui navigation systems until the user sends a
  `NavRequest::Free`. See the `locking.rs` example for illustration.
  * Breaking: New enum variants on `NavRequest` and `NavEvent`
* `0.11.1`: Add the `marker` module, enabling propagation of user-specified
  components to `Focusable` children of a `NavMenu`.
* `0.12.0`: Remove `NavMenu` methods from `MarkingMenu` and make the `menu`
  field public instead. Internally, this represented too much duplicate code.
* `0.12.1`: Add *by-name* menus, making declaring complex menus in one go much easier.
* `0.13.0`: Complete rewrite of the [`NavMenu`] declaration system:
  * Add automatic submenu access for `scope` menus.
  * Rename examples, they were named weirdly.
  * **Breaking**: Replace `NavMenu` constructor API with an enum (KISS) and a
    set of methods that return various types of `Bundle`s. Each variant does
    what the `cycle` and `scope` methods used to do.
  * **Breaking**: `NavMenu` is not a component anymore, the one used in the
    navigation algorithm is now private, you can't match on `NavMenu` in query
    parameters anymore. If you need that functionality, create your own marker
    component and add it manually to your menu entities.
  * **Breaking**: Remove `is_*` methods from `Focusable`. Please use the
    `state` method instead. The resulting program will be more correct. If you
    are only worried about discriminating the `Focused` element from others,
    just use a `if let Focused = focus.state() {} else {}`. Please see the
    examples directory for usage patterns.
  * **Breaking**: A lot of struct and module reordering to make documentation
    more discoverable. Notably `Direction` and `ScopeDirection` are now in the
    `events` module.
* `0.13.1`: Fix broken URLs in Readme.md
* `0.14.0`: Some important changes, and a bunch of new very useful features.
  * Add a [`Focusable::dormant`] constructor to specify which focusable you want
    to be the first to focus, this also works for [`Focusable`]s within
    [`NavMenu`]s.
  * **Important**: This changes the library behavior, now there will
    automatically be a `Focused` entity set. Add a system to set the first
    `Focused` whenever `Focusable`s are added to the world.
  * Add [`NavEvent::InitiallyFocused`] to handle this first `Focused` event.
  * Early-return in `default_gamepad_input` and `default_keyboard_input` when
    there are no `Focusable` elements in the world. This saves your precious
    CPU cycles. And prevents spurious `warn` log messages.
  * Do not crash when resolving `NavRequest`s while no `Focusable`s exists in
    the world. Instead, it now prints a warning message.
  * **Important**: Now the focus handling algorithm supports multiple `NavRequest`s
    per frame. If previously you erroneously sent multiple `NavRequest` per
    update and relied on the ignore mechanism, you'll have a bad time.
  * This also means the focus changes are visible as soon as the system ran,
    the new `NavRequestSystem` label can be used to order your system in
    relation to the focus update system. **This makes the focus change much
    snappier**.
  * Rewrite the `ultimate_menu_navigation.rs` without the `build_ui!` macro
    because we shouldn't expect users to be familiar with my personal weird
    macro.
  * **Breaking**: Remove `Default` impl on `NavLock`. The user shouldn't be
    able to mutate it, you could technically overwrite the `NavLock` resource
    by using `insert_resource(NavLock::default())`.
* `0.15.0`: **Breaking**: bump bevy version to `0.7` (you should be able to
    upgrade from `0.14.0` without changing your code)
* `0.15.1`: Fix the `marker` systems panicking at startup.
* `0.16.0`:
  * Cycling now wraps around the screen properly, regardless of UI camera
    position and scale. See the new `off_screen_focusables.rs` example for a demo.
  * Fix the `default_mouse_input` system not accounting for camera scale.
  * Update examples to make use of `NavRequestSystem` label, add more recommendations
    with regard to system ordering.
  * **Warning**: if you have some funky UI that goes beyond the screen (which
    you are likely to have if you use the `Overflow` feature), this might result
    in unexpected behavior. Please fill a bug if you hit that limitation.
  * Add a nice stress test example with 96000 focusable nodes. This crate is not
    particularly optimized, but it's nice to see it holds up!
  * **Breaking**: Removed the undocumented public `UiCamera` marker component, please
    use the bevy native `bevy::ui::entity::CameraUi` instead. Furthermore, the
    `default_mouse_input` system has one less parameter.
  * **Warning**: if you have multiple UI camera, things will definitively break. Please
    fill an issue if you stumble uppon that case.
* `0.17.0`: Non-breaking, but due to cargo semver handling is a minor bump.
  * Add the `event_helpers` module to simplify ui event handling
  * Fix README and crate-level documentation links
* `0.18.0`:
  * **Breaking**: Remove marker generic type parameter from `systems::NodePosQuery`.
    The [`generic_default_mouse_input`] system now relies on the newly introduced
    [`ScreenBoundaries`] that abstracts camera offset and zoom settings.
  * **Important**: Introduced new plugins to make it even simpler to get started.
  * Add `event_helpers` module introduction to README.
  * Fix `bevy-ui` feature not building. This issue was introduced in `0.16.0`.
* `0.19.0`: **Breaking**: Update to bevy 0.8.0
  * **Warning**: 0.8.0 removed the ability for the user to change the ui camera position
    and perspective, see <https://github.com/bevyengine/bevy/pull/5252>
    Generic support for user-defined UIs still allows custom cropping, but it not a relevant
    use case to the default bevy_ui library.
  * Keyboard navigation in the style of games pre-dating computer mouses is now disabled by default.
    While you can still use the escape and tab keys for interaction, you cannot use keyboard keys
    to navigate between focusables anymore, this prevents keyboard input conflicts.
    You can enable keyboard movement using the [`InputMapping::keyboard_navigation`] field.
  * Touch input handling has been removed, it was untested and probably broken, better let
    the user who knows what they are doing do it.
  * **NEW**: Add complete user-customizable focus movement. Now it should be possible to implement
    focus navigation in 3d space.
  * **Breaking**: This requires making the plugin generic over the navigation system, if you were
    manually adding `NavigationPlugin`, please consider using `DefaultNavigationPlugins` instead,
    if it is not possible, then use `NavigationPlugin::new()`.

[`ButtonBundle`]: https://docs.rs/bevy/0.8.0/bevy/ui/entity/struct.ButtonBundle.html
[Changed]: https://docs.rs/bevy/0.8.0/bevy/ecs/prelude/struct.Changed.html
[doc-root]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/
[`Entity`]: https://docs.rs/bevy/0.8.0/bevy/ecs/entity/struct.Entity.html
[entity-id]: https://docs.rs/bevy/0.8.0/bevy/ecs/system/struct.EntityCommands.html#method.id
[example-marking]: https://github.com/nicopap/ui-navigation/tree/v0.19.0/examples/marking.rs
[examples]: https://github.com/nicopap/ui-navigation/tree/v0.19.0/examples
[example-simple]: https://github.com/nicopap/ui-navigation/tree/v0.19.0/examples/simple.rs
[example-ultimate]: https://github.com/nicopap/ui-navigation/blob/v0.19.0/examples/ultimate_menu_navigation.rs
[`FocusableButtonBundle`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/components/struct.FocusableButtonBundle.html
[`Focusable::cancel`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.Focusable.html#method.cancel
[`Focusable::dormant`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.Focusable.html#method.dormant
[`Focusable`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.Focusable.html
[`Focusable::lock`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.Focusable.html#method.lock
[`generic_default_mouse_input`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/systems/fn.generic_default_mouse_input.html
[`InputMapping`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/systems/struct.InputMapping.html
[module-event_helpers]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/event_helpers/index.html
[module-marking]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/bundles/struct.MenuSeed.html#method.marking
[module-systems]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/systems/index.html
[Name]: https://docs.rs/bevy/0.8.0/bevy/core/enum.Name.html
[`NavEvent::FocusChanged`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavEvent.html#variant.FocusChanged
[`NavEvent`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavEvent.html
[`NavEvent::InitiallyFocused`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavEvent.html#variant.InitiallyFocused
[`NavMenu`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/enum.NavMenu.html
[NavMenu::reachable_from]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/enum.NavMenu.html#method.reachable_from
[NavMenu::reachable_from_named]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/enum.NavMenu.html#method.reachable_from_named
[`NavRequest`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavRequest.html
[`NavRequest::Action`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavRequest.html#variant.Action
[`NavRequest::FocusOn`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavRequest.html#variant.FocusOn
[`NavRequest::Free`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavRequest.html#variant.Free
[`NavRequest::ScopeMove`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/events/enum.NavRequest.html#variant.ScopeMove
[`NavRequestSystem`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.NavRequestSystem.html
[rfc41]: https://github.com/nicopap/rfcs/blob/ui-navigation/rfcs/41-ui-navigation.md
[`ScreenBoundaries`]: https://docs.rs/bevy-ui-navigation/0.19.0/bevy_ui_navigation/struct.ScreenBoundaries.html


### Version matrix

| bevy | latest supporting version      |
|------|--------|
| 0.8  | 0.19.0 |
| 0.7  | 0.18.0 |
| 0.6  | 0.14.0 |

### Notes on API Stability

In the 4th week of January, there has been 5 breaking version changes. `0.13.0`
marks the end of this wave of API changes. And things should go much slower in
the future.

The new `NavMenu` construction system helps adding orthogonal features to the
library without breaking API changes. However, since bevy is still in `0.*`
territory, it doesn't make sense for this library to leave the `0.*` range.

Also, the way cargo handles versioning for `0.*` crates is in infraction of
the semver specification. Meaning that additional features without breakages
requires bumping the minor version rather than the patch version (as should
pre-`1.` versions do).


# License

Copyright © 2022 Nicola Papale

This software is licensed under either MIT or Apache 2.0 at your leisure. See
licenses directory for details.
