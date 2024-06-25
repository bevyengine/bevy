//! Everything we need to handle json output and input.

use serde::Serialize;
use serde_json::{Map, Value};

pub fn message_format_option(emit_json: bool) -> &'static str {
    if emit_json {
        "--message-format=json"
    } else {
        "--message-format=human"
    }
}

#[derive(Debug, Serialize)]
pub struct JsonCommandOutput {
    pub command_name: String,
    pub messages: Vec<Map<String, Value>>,
}

impl JsonCommandOutput {
    pub fn from_cargo_output(output: Vec<u8>, command_name: String) -> Option<JsonCommandOutput> {
        /// Used to filter out compiler messages we don't care about
        ///
        /// The cargo json output format is rather verbose and tends to produce
        /// long lines. It would be expensive to deserialize them all especially
        /// when for most of them we'll check one field and then discard them.
        const COMPILER_MESSAGE_FILTER: &str = r#""reason":"compiler-message""#;

        let json_string = std::str::from_utf8(&output).ok()?;

        let mut messages = vec![];

        // Each line is an individual JSON object that can be parsed separately.
        for line in json_string.lines() {
            if !line.contains(COMPILER_MESSAGE_FILTER) {
                continue;
            }

            // Parse the JSON, silently skipping it on failure.
            let Ok(mut message) = serde_json::from_str::<Map<String, Value>>(line) else {
                continue;
            };

            // Retrieve the message, skipping it if unavailable.
            let Some(Value::Object(inner_message)) = message.remove("message") else {
                continue;
            };

            messages.push(inner_message);
        }

        Some(JsonCommandOutput {
            command_name,
            messages,
        })
    }
}
