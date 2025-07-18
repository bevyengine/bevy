use bevy::prelude::*;
use std::collections::HashSet;

/// The state of the inspector's tree view.
#[derive(Resource, Default)]
pub struct TreeState {
    pub expanded_nodes: HashSet<String>,
}

/// An event that signals an interaction with a tree node.
#[derive(Event, Debug)]
pub struct TreeNodeInteraction {
    pub node_id: String,
}

impl bevy::ecs::event::BufferedEvent for TreeNodeInteraction {}
