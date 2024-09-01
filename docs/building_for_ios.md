# Building Bevy for iOS

This guide will walk you through the process of building a Bevy game for iOS. The steps involve compiling your game as a static library for the iOS architecture, creating an Xcode project, linking to the static library along with the necessary Apple frameworks, and finally calling your game’s entry point function from Objective-C.

## Requirements

- Make sure that XCode is installed
- Install the iOS target with rustup

```bash
rustup target add aarch64-apple-ios
```

## Defining the game entry point

The typical entry point for a Bevy game is the main function in main.rs. However, for iOS, we need to be able to call the entry function from Objective-C’s main function.

Create `src/lib.rs` and define the entry point.

```rust
#[no_mangle]
pub extern fn my_game_entry(){
    // ... Your game code
}
```

## Build your game as a static library for iOS

In `Cargo.toml`, ensure you are building a `staticlib`

```toml
[lib]
crate-type = ["lib", "staticlib"]
```

Now you can compile the static library

```bash
cargo build --release --target=aarch64-apple-ios
```

The resulting artifact with be in `./target/release/libmy_bevy_game.a`

## Create the XCode project

Now create an XCode project using the iOS App template, be sure to select Objective-C as the language.

- Remove all .h and .m files except for main.m.
- Remove the Main storyboard file
- Remove the `Main storyboard file base name` entry from the Info.plist
- Replace the contents of main.m with the following

```c
extern void my_game_entry(void);

int main(int argc, const char * argv[]) {
    my_game_entry();
    return 0;
}
```

## Linking to the game library

Now you can link your app target to your rust static library.

- In your project’s "Build Settings" search for "Other Linker Flags."
- Add the absolute path to your build artifact in this section, for example:
`~/repos/my_bevy_game/target/aarch64-apple-ios/release/libmy_bevy_game.a`

In your project’s "Build Phases," under "Link Binary With Libraries," add the following frameworks:

- UIKit
- Metal
- AudioToolbox

You should now be able to build and run your game on a real iOS device (Not the simulator)
