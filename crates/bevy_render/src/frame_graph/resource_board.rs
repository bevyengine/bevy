use std::collections::HashMap;

use super::GraphRawResourceNodeHandle;

#[derive(PartialEq, Eq, Hash)]
pub struct ResourceBoardKey(String);

impl<'a> From<&'a str> for ResourceBoardKey {
    fn from(s: &'a str) -> Self {
        ResourceBoardKey(String::from(s))
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
    pub fn put<T: Into<ResourceBoardKey>>(&mut self, key: T, handle: GraphRawResourceNodeHandle) {
        let key = key.into();
        self.resources.insert(key, handle);
    }

    pub fn get<T: Into<ResourceBoardKey>>(&self, key: T) -> Option<&GraphRawResourceNodeHandle> {
        let key = key.into();
        self.resources.get(&key)
    }
}
