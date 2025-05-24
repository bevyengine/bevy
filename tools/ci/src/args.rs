/// Arguments that are available to CI commands.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Args {
    pub keep_going: bool,
    pub test_threads: Option<usize>,
    pub build_jobs: Option<usize>,
}
