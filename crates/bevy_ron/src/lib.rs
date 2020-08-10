/*!
RON is a simple config format which looks similar to Rust syntax.

## Features

* Data types
    * Structs, typename optional
    * Tuples
    * Enums
    * Lists
    * Maps
    * Units (`()`)
    * Optionals
    * Primitives: booleans, numbers, string, char
* Allows nested layout (similar to JSON)
* Supports comments
* Trailing commas
* Pretty serialization

## Syntax example

```rust,ignore
Game(
    title: "Hello, RON!",
    level: Level( // We could just leave the `Level` out
        buildings: [
            (
                size: (10, 20),
                color: Yellow, // This as an enum variant
                owner: None,
            ),
            (
                size: (20, 25),
                color: Custom(0.1, 0.8, 1.0),
                owner: Some("guy"),
            ),
        ],
        characters: {
            "guy": (
                friendly: true,
            ),
        },
    ),
)
```

## Usage

Just add it to your `Cargo.toml`:

```toml
[dependencies]
ron = "*"
```

Serializing / Deserializing is as simple as calling `to_string` / `from_str`.

!*/

#![doc(html_root_url = "https://docs.rs/ron/0.6.0")]

pub mod de;
pub mod ser;

pub mod error;
pub mod value;

pub mod extensions;

pub use de::{from_str, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_string, Serializer};
pub use value::{Map, Number, Value};

mod parse;
