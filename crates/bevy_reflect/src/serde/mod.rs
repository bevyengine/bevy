mod de;
mod ser;

pub use de::*;
pub use ser::*;

pub(crate) mod type_fields {
    pub const TYPE: &str = "type";
    pub const VALUE: &str = "value";
}
