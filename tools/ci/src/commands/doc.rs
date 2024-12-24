use crate::commands::{DocCheckCommand, DocTestCommand};
use argh::FromArgs;

/// Alias for running the `doc-test` and `doc-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc")]
pub struct DocCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl DocCommand {
    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        let mut jsons = Vec::new();

        let doc_test_result = if self.emit_json {
            DocTestCommand::run_with_intermediate_json(no_fail_fast).map(|json| jsons.push(json))
        } else {
            DocTestCommand::run_with_intermediate(no_fail_fast)
        };

        if !no_fail_fast && doc_test_result.is_err() {
            if self.emit_json {
                println!("{}", serde_json::to_string(&jsons).unwrap());
            }
            return doc_test_result;
        }

        let doc_check_result = if self.emit_json {
            DocCheckCommand::run_with_intermediate_json().map(|json| jsons.push(json))
        } else {
            DocCheckCommand::run_with_intermediate()
        };

        if self.emit_json {
            println!("{}", serde_json::to_string(&jsons).unwrap());
        }

        doc_check_result
    }
}
