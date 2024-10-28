#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Args {
    pub keep_going: bool,
    pub test_threads: Option<u8>,
}
