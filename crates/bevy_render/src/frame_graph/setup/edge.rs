use crate::render_graph::InternedRenderLabel;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Edge {
    /// An edge describing to ordering of both nodes (`output_node` before `input_node`)
    /// and connecting the output slot at the `output_index` of the `output_node`
    /// with the slot at the `input_index` of the `input_node`.
    SlotEdge {
        input_node: InternedRenderLabel,
        input_index: usize,
        output_node: InternedRenderLabel,
        output_index: usize,
    },
    /// An edge describing to ordering of both nodes (`output_node` before `input_node`).
    NodeEdge {
        input_node: InternedRenderLabel,
        output_node: InternedRenderLabel,
    },
}

impl Edge {
    /// Returns the id of the `input_node`.
    pub fn get_input_node(&self) -> InternedRenderLabel {
        match self {
            Edge::SlotEdge { input_node, .. } | Edge::NodeEdge { input_node, .. } => *input_node,
        }
    }

    /// Returns the id of the `output_node`.
    pub fn get_output_node(&self) -> InternedRenderLabel {
        match self {
            Edge::SlotEdge { output_node, .. } | Edge::NodeEdge { output_node, .. } => *output_node,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum EdgeExistence {
    Exists,
    DoesNotExist,
}
