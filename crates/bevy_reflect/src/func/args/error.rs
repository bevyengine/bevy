use alloc::borrow::Cow;

use thiserror::Error;

use crate::func::args::{ArgId, Ownership};

#[derive(Debug, Error, PartialEq)]
pub enum ArgError {
    #[error("expected `{expected}` but received `{received}` (@ {id:?})")]
    UnexpectedType {
        id: ArgId,
        expected: Cow<'static, str>,
        received: Cow<'static, str>,
    },
    #[error("expected {expected} value but received {received} value (@ {id:?})")]
    InvalidOwnership {
        id: ArgId,
        expected: Ownership,
        received: Ownership,
    },
}
