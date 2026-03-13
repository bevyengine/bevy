use crate::{args::Args, PreparedCommand};
use argh::SubCommand;
use xshell::cmd;

/// Prepares the clippy permutations of a crate.
pub(super) struct ClippyPermutations {
    /// Crate name
    pub crate_name: &'static str,
    /// Features used when running clippy with no default features
    pub features: &'static [&'static str],
    /// Features used when running clippy with all features
    pub all_features_features: &'static [&'static str],
}

impl ClippyPermutations {
    pub fn build<'a, T: SubCommand>(
        self,
        sh: &'a xshell::Shell,
        args: Args,
    ) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();
        let jobs_ref = jobs.as_ref();
        let target = args.target();
        let target_ref = target.as_ref();
        let crate_name = self.crate_name;

        let mut permutations =
            Vec::with_capacity(2 + self.features.len() + self.all_features_features.len().min(1));

        // No default features
        permutations.push(PreparedCommand::new::<T>(
            cmd!(
                sh,
                "cargo clippy -p {crate_name} --no-default-features {jobs_ref...} {target_ref...} -- -Dwarnings"
            ),
            "Please fix clippy errors in output above.",
        ));
        // Feature permutations
        for feature in self.features {
            permutations.push(PreparedCommand::new::<T>(
                cmd!(
                    sh,
                    "cargo clippy -p {crate_name} --no-default-features --features={feature} {jobs_ref...} {target_ref...} -- -Dwarnings"
                ),
                "Please fix clippy errors in output above.",
            ));
        }
        // Default features
        permutations.push(PreparedCommand::new::<T>(
            cmd!(
                sh,
                "cargo clippy -p {crate_name} {jobs_ref...} {target_ref...} -- -Dwarnings"
            ),
            "Please fix clippy errors in output above.",
        ));
        // All features
        if self.all_features_features.is_empty() {
            permutations.push(PreparedCommand::new::<T>(
                cmd!(
                    sh,
                    "cargo clippy -p {crate_name} --all-features {jobs_ref...} {target_ref...} -- -Dwarnings"
                ),
                "Please fix clippy errors in output above.",
            ));
        } else {
            for feature in self.all_features_features {
                permutations.push(PreparedCommand::new::<T>(
                    cmd!(
                        sh,
                        "cargo clippy -p {crate_name} --all-features --features={feature} {jobs_ref...} {target_ref...} -- -Dwarnings"
                    ),
                    "Please fix clippy errors in output above.",
                ));
            }
        }

        permutations
    }
}
