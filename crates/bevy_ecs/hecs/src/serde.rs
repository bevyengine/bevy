// modified by Bevy contributors

use crate::entities::Entity;
use serde::{Serialize, Serializer};

impl Serialize for Entity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(self.id())
    }
}
