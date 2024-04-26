/// This macro simplifies declaring new commands and aliases.
/// 
/// # Example
/// 
/// ```rust,no_compile
/// // This macro...
/// define_commands! {
///     commands: [
///         test::TestCommand,
///     ],
///     alias: [
///         build: BuildCommand,
///     ],
/// }
/// 
/// // ...generates this.
/// mod test;
/// pub(crate) use test::TestCommand;
/// 
/// mod build;
/// pub(crate) use build::BuildCommand;
/// ```
macro_rules! define_commands {
    {
        commands: [
            $($command_module:ident::$command:ident),* $(,)?
        ],
        aliases: [
            $($alias_module:ident::$alias:ident),* $(,)?
        ] $(,)?
    } => {
        $(
            mod $command_module;
            pub(crate) use $command_module::$command;
        )*

        $(
            mod $alias_module;
            pub(crate) use $alias_module::$alias;
        )*
    };
}

define_commands! {
    commands: [
        bench_check::BenchCheckCommand,
        cfg_check::CfgCheckCommand,
        clippy::ClippyCommand,
        compile_check::CompileCheckCommand,
        compile_fail::CompileFailCommand,
        doc_check::DocCheckCommand,
        doc_test::DocTestCommand,
        example_check::ExampleCheckCommand,
        format::FormatCommand,
        test::TestCommand,
        test_check::TestCheckCommand,
    ],
    aliases: [
        compile::CompileCommand,
        doc::DocCommand,
        lints::LintsCommand,
    ],
}
