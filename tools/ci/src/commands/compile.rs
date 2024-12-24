use crate::commands::{
    BenchCheckCommand, CompileCheckCommand, CompileFailCommand, ExampleCheckCommand,
    TestCheckCommand,
};
use argh::FromArgs;

/// Alias for running the `compile-fail`, `bench-check`, `example-check`, `compile-check`, and `test-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile")]
pub struct CompileCommand {}

impl CompileCommand {
    /// Runs this command.
    pub fn run(self, no_fail_fast: bool) -> Result<(), ()> {
        let compile_fail_result = CompileFailCommand::run_with_intermediate(no_fail_fast);

        if !no_fail_fast && compile_fail_result.is_err() {
            return compile_fail_result;
        }

        let bench_check_result = BenchCheckCommand::run_with_intermediate();

        if !no_fail_fast && compile_fail_result.is_err() {
            return bench_check_result;
        }

        let example_check_result = ExampleCheckCommand::run_with_intermediate();

        if !no_fail_fast && compile_fail_result.is_err() {
            return example_check_result;
        }

        let compile_check_result = CompileCheckCommand::run_with_intermediate();

        if !no_fail_fast && compile_fail_result.is_err() {
            return compile_check_result;
        }

        TestCheckCommand::run_with_intermediate()
    }
}
