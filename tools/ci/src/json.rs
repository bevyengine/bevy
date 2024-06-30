//! Everything we need to handle json output and input.

use std::{
    error::Error,
    io::{BufRead, BufReader, Read},
};

use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct JsonCommandOutput {
    pub command_name: String,
    pub messages: Vec<Map<String, Value>>,
}

impl JsonCommandOutput {
    /// Parses
    pub fn from_cargo_output(
        output: impl Read,
        command_name: String,
    ) -> Result<JsonCommandOutput, Box<dyn Error>> {
        /// Used to filter out compiler messages we don't care about
        ///
        /// The cargo json output format is rather verbose and tends to produce
        /// long lines. It would be expensive to deserialize them all especially
        /// when for most of them we'll check one field and then discard them.
        const COMPILER_MESSAGE_FILTER: &str = r#""reason":"compiler-message""#;

        let reader = BufReader::new(output);

        let mut messages = vec![];

        // Each line is an individual JSON object that can be parsed separately.
        for line in reader.lines() {
            let line = line?;

            if !line.contains(COMPILER_MESSAGE_FILTER) {
                continue;
            }

            let mut message = serde_json::from_str::<Map<String, Value>>(&line)?;

            let Some(Value::Object(inner_message)) = message.remove("message") else {
                // We don't panic or directly print the error because we're likely running a cargo process in the
                // background that has inherited our stderr. Writing to stderr might clash with it's error output
                // and we want to wait for it to finish when something goes wrong.
                return Err("Cargo message didn't contain a `message` key".into());
            };

            messages.push(inner_message);
        }

        Ok(JsonCommandOutput {
            command_name,
            messages,
        })
    }

    pub fn as_json_string(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|err| {
            unreachable!("serde_json should never fail to serialize this! Failure reason: {err}")
        })
    }
}
