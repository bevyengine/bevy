use serde::{Deserialize, Serialize};

bitflags::bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct Extensions: usize {
        const UNWRAP_NEWTYPES = 0x1;
        const IMPLICIT_SOME = 0x2;
    }
}

impl Extensions {
    /// Creates an extension flag from an ident.
    pub fn from_ident(ident: &[u8]) -> Option<Extensions> {
        match ident {
            b"unwrap_newtypes" => Some(Extensions::UNWRAP_NEWTYPES),
            b"implicit_some" => Some(Extensions::IMPLICIT_SOME),
            _ => None,
        }
    }
}

impl Default for Extensions {
    fn default() -> Self {
        Extensions::empty()
    }
}
