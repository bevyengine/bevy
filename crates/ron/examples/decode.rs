use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Config {
    boolean: bool,
    float: f32,
    map: HashMap<u8, char>,
    nested: Nested,
    option: Option<String>,
    tuple: (u32, u32),
}

#[derive(Debug, Deserialize)]
struct Nested {
    a: String,
    b: char,
}

const CONFIG: &str = "
/*
 * RON now has multi-line (C-style) block comments!
 * They can be freely nested:
 * /* This is a nested comment */
 * If you just want a single-line comment,
 * do it like here:
// Just put two slashes before the comment and the rest of the line
// can be used freely!
*/

// Note that block comments can not be started in a line comment
// (Putting a /* here will have no effect)

(
    boolean: true,
    float: 8.2,
    map: {
        1: '1',
        2: '4',
        3: '9',
        4: '1',
        5: '2',
        6: '3',
    },
    nested: Nested(
        a: \"Decode me!\",
        b: 'z',
    ),
    option: Some(\t  \"Weird formatting!\" \n\n ),
    tuple: (3 /*(2 + 1)*/, 7 /*(2 * 5 - 3)*/),
)";

fn main() {
    let config: Config = match from_str(CONFIG) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            std::process::exit(1);
        }
    };

    println!("Config: {:?}", &config);
}
