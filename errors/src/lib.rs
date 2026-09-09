//! Definitions of Bevy's error codes that might occur at runtime.
//!
//! These either manifest as a warning or a panic.

#[doc = include_str!("../B0001.md")]
pub struct B0001;

#[doc = include_str!("../B0002.md")]
pub struct B0002;

#[doc = include_str!("../B0003.md")]
pub struct B0003;

#[doc = include_str!("../B0004.md")]
pub struct B0004;

#[doc = include_str!("../B0005.md")]
pub struct B0005;

#[expect(
    clippy::needless_doctest_main,
    reason = "The example is emphasizing that the code should go in the user's `fn main()`."
)]
#[doc = include_str!("../B0006.md")]
pub struct B0006;

#[doc = include_str!("../B0007.md")]
pub struct B0007;

#[doc = include_str!("../B0008.md")]
pub struct B0008;

#[doc = include_str!("../B0009.md")]
pub struct B0009;
