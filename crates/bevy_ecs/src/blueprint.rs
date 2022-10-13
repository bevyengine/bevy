use crate::system::EntityCommands;

pub trait EntityBlueprint {
    fn build(self, entity: &mut EntityCommands);
}
