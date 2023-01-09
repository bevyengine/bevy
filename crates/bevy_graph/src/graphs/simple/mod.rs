mod map;
pub use map::*;

mod list;
pub use list::*;

use slotmap::new_key_type;

new_key_type! {
    pub struct NodeIdx;
    pub struct EdgeIdx;
}
