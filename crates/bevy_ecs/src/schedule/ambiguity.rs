use core::{
    fmt::Write as _,
    ops::{Deref, DerefMut},
};

use alloc::string::{String, ToString};
use bevy_platform::collections::HashSet;

use crate::{
    component::{ComponentId, Components},
    resource::Resource,
};

/// List of [`ComponentId`]s to ignore when reporting system order ambiguity conflicts.
#[derive(Resource, Default)]
pub struct IgnoredAmbiguities(pub HashSet<ComponentId>);

impl IgnoredAmbiguities {
    /// Returns a string listing all ignored ambiguity component names.
    ///
    /// May panic or retrieve incorrect names if [`Components`] is not from the
    /// same world.
    pub fn to_string(&self, components: &Components) -> String {
        let mut message =
            "System order ambiguities caused by conflicts on the following types are ignored:\n"
                .to_string();
        for id in self.iter() {
            writeln!(message, "{}", components.get_name(*id).unwrap()).unwrap();
        }
        message
    }
}

impl Deref for IgnoredAmbiguities {
    type Target = HashSet<ComponentId>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for IgnoredAmbiguities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
