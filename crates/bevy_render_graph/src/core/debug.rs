use super::{
    resource::{RenderHandle, RenderResource},
    Label, RenderGraph,
};

pub trait RenderGraphDebug<'g>: Sized {
    fn fmt(
        &self,
        ctx: RenderGraphDebugContext<'_, 'g>,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result;
}

impl<'g, T: std::fmt::Debug> RenderGraphDebug<'g> for T {
    fn fmt(
        &self,
        _ctx: RenderGraphDebugContext<'_, 'g>,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

#[derive(Copy, Clone)]
pub struct RenderGraphDebugContext<'a, 'g: 'a>(pub(super) &'a RenderGraph<'g>);

impl<'a, 'g: 'a> RenderGraphDebugContext<'a, 'g> {
    pub fn label<R: RenderResource>(&self, resource: RenderHandle<'g, R>) -> &Label<'g> {
        self.0.label(resource.id())
    }

    pub fn debug<T: RenderGraphDebug<'g>>(
        self,
        value: &'a T,
    ) -> RenderGraphDebugWrapper<'a, 'g, T> {
        RenderGraphDebugWrapper(self, value)
    }
}

pub struct RenderGraphDebugWrapper<'a, 'g: 'a, T: RenderGraphDebug<'g>>(
    RenderGraphDebugContext<'a, 'g>,
    &'a T,
);

impl<'a, 'g: 'a, T: RenderGraphDebug<'g>> std::fmt::Debug for RenderGraphDebugWrapper<'a, 'g, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.1.fmt(self.0, f)
    }
}
