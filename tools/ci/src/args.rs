use crate::CI;

/// Arguments that are available to CI commands.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Args {
    keep_going: bool,
    test_threads: Option<usize>,
    build_jobs: Option<usize>,
}

impl Args {
    #[inline(always)]
    pub fn keep_going(&self) -> Option<&'static str> {
        self.keep_going.then_some("--no-fail-fast")
    }

    #[inline(always)]
    pub fn build_jobs(&self) -> Option<String> {
        self.build_jobs.map(|jobs| format!("--jobs={jobs}"))
    }

    #[inline(always)]
    pub fn test_threads(&self) -> Option<String> {
        self.test_threads
            .map(|threads| format!("--test-threads={threads}"))
    }
}

impl From<&CI> for Args {
    fn from(value: &CI) -> Self {
        Args {
            keep_going: value.keep_going,
            test_threads: value.test_threads,
            build_jobs: value.build_jobs,
        }
    }
}
