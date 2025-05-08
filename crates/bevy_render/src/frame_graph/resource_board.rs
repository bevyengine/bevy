use std::collections::HashMap;

use super::GraphRawResourceNodeHandle;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct ResourceBoardKey(String);

impl<'a> From<&'a str> for ResourceBoardKey {
    fn from(s: &'a str) -> Self {
        ResourceBoardKey(String::from(s))
    }
}

impl<'a> From<&'a ResourceBoardKey> for ResourceBoardKey {
    fn from(s: &'a ResourceBoardKey) -> Self {
        s.to_owned()
    }
}

impl<'a> From<&'a String> for ResourceBoardKey {
    fn from(s: &'a String) -> Self {
        ResourceBoardKey(s.to_string())
    }
}

impl From<String> for ResourceBoardKey {
    fn from(s: String) -> Self {
        ResourceBoardKey(s)
    }
}
#[derive(Default)]
pub struct ResourceBoard {
    resources: HashMap<ResourceBoardKey, GraphRawResourceNodeHandle>,
}

impl ResourceBoard {
    pub fn put(&mut self, key: ResourceBoardKey, handle: GraphRawResourceNodeHandle) {
        self.resources.insert(key, handle);
    }

    pub fn get(&self, key: &ResourceBoardKey) -> Option<&GraphRawResourceNodeHandle> {
        self.resources.get(&key)
    }
}
