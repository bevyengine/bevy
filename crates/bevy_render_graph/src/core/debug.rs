use super::RenderGraph;

pub trait RenderGraphDebug<'g>: Sized {
    fn fmt(&self, graph: &RenderGraph<'g>, f: &mut std::fmt::Formatter) -> std::fmt::Result;

    fn debug<'a>(&'a self, graph: &'a RenderGraph<'g>) -> RenderGraphDebugContext<'a, 'g, Self> {
        RenderGraphDebugContext(graph, self)
    }
}

impl<'g, T: std::fmt::Debug> RenderGraphDebug<'g> for T {
    fn fmt(&self, _graph: &RenderGraph<'g>, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub struct RenderGraphDebugContext<'a, 'g: 'a, T: RenderGraphDebug<'g>>(&'a RenderGraph<'g>, &'a T);

impl<'a, 'g: 'a, T: RenderGraphDebug<'g>> std::fmt::Debug for RenderGraphDebugContext<'a, 'g, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.1.fmt(self.0, f)
    }
}
