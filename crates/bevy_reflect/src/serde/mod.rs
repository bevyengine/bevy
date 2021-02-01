mod de;
mod ser;

pub use de::*;
pub use ser::*;

pub(crate) mod type_fields {
    pub const TYPE: &str = "type";
    pub const MAP: &str = "map";
    pub const STRUCT: &str = "struct";
    pub const TUPLE_STRUCT: &str = "tuple_struct";
    pub const TUPLE: &str = "tuple";
    pub const LIST: &str = "list";
    pub const VALUE: &str = "value";
}
