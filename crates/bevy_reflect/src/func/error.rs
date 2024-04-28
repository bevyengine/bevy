use crate::func::args::ArgError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum FuncError {
    #[error(transparent)]
    Arg(#[from] ArgError),
    #[error("expected {expected} arguments but received {received}")]
    ArgCount { expected: usize, received: usize },
}
