use bevy_ecs::entity::Entity;
use std::borrow::Cow;
use thiserror::Error;

/// A command that signals the graph runner to run the sub graph corresponding to the `name`
/// with the specified `inputs` next.
pub struct RunSubGraph {
    pub name: Cow<'static, str>,
    pub view_entity: Entity,
}

/// The context with all graph information required to run a [`Node`](super::Node).
/// This context is created for each node by the `RenderGraphRunner`.
///
/// The slot input can be read from here and the outputs must be written back to the context for
/// passing them onto the next node.
///
/// Sub graphs can be queued for running by adding a [`RunSubGraph`] command to the context.
/// After the node has finished running the graph runner is responsible for executing the sub graphs.
pub struct RenderGraphContext {
    run_sub_graphs: Vec<RunSubGraph>,
    view_entity: Option<Entity>,
}

impl RenderGraphContext {
    /// Creates a new render graph context for the `node`.
    pub fn new() -> Self {
        Self {
            run_sub_graphs: Vec::new(),
            view_entity: None,
        }
    }

    pub fn view_entity(&self) -> Entity {
        self.view_entity.unwrap()
    }

    pub fn get_view_entity(&self) -> Option<Entity> {
        self.view_entity
    }

    pub fn set_view_entity(&mut self, view_entity: Entity) {
        self.view_entity = Some(view_entity);
    }

    /// Queues up a sub graph for execution after the node has finished running.
    pub fn run_sub_graph(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        view_entity: Entity,
    ) -> Result<(), RunSubGraphError> {
        let name = name.into();
        self.run_sub_graphs.push(RunSubGraph { name, view_entity });
        Ok(())
    }

    /// Finishes the context for this [`Node`](super::Node) by
    /// returning the sub graphs to run next.
    pub fn finish(self) -> Vec<RunSubGraph> {
        self.run_sub_graphs
    }
}

impl Default for RenderGraphContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Error, Debug, Eq, PartialEq)]
pub enum RunSubGraphError {
    #[error("attempted to run sub-graph `{0}`, but it does not exist")]
    MissingSubGraph(Cow<'static, str>),
}
