use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Generates the documentation with the same configuration as docs.rs and dev-docs. This requires
/// nightly Rust.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-generate")]
pub struct DocGenerateCommand {
    /// file to include inline in the <head> section of the generated documentation
    // This is included largely for building the dev-docs, which disables indexing using this.
    #[argh(option)]
    html_in_header: Option<String>,
}

impl Prepare for DocGenerateCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut rustdoc_flags = "-Zunstable-options --cfg=docsrs".to_string();

        // If `html_in_header` is specified, add it to the rustdoc flags.
        if let Some(ref file) = self.html_in_header {
            rustdoc_flags.push_str(" --html-in-header ");
            rustdoc_flags.push_str(file);
        }

        // TODO: Find alternative solution
        // Leak string, since `PreparedCommand` requires 'static lifetime.
        let rustdoc_flags: &'static str = rustdoc_flags.leak();

        vec![PreparedCommand::new::<Self>(
            cmd!(
                sh,
                "cargo +nightly doc -p bevy --all-features --no-deps -Zunstable-options -Zrustdoc-scrape-examples"
            ),
            "Failed to build documentation. (This requires nightly Rust, please ensure it is installed!).",
        ).with_env_var("RUSTDOCFLAGS", rustdoc_flags)]
    }
}
